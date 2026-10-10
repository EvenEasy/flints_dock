use super::*;
use crate::app::swap::{SwapError, SwapProvider, SwapRequest};
pub use crate::core::cleanup::unsupported_reason;

/// Return account-scoped cleanup inputs from the snapshot mint and classification indexes.
/// Unmatched classifications remain `Unknown`; balances are never aggregated across accounts.
pub fn assets_from_portfolio(portfolio: &WalletSnapshot) -> Vec<CleanupAsset> {
    let mints: std::collections::BTreeMap<_, _> = portfolio
        .mints
        .iter()
        .map(|mint| ((mint.mint.as_str(), mint.program), mint))
        .collect();
    let kinds: std::collections::BTreeMap<_, _> = portfolio
        .all_tokens
        .iter()
        .flat_map(|asset| {
            asset
                .accounts
                .iter()
                .map(move |account| (account.as_str(), asset.kind))
        })
        .collect();
    portfolio
        .token_accounts
        .iter()
        .map(|account| CleanupAsset {
            account: account.clone(),
            mint: mints
                .get(&(account.mint.as_str(), account.program))
                .map(|mint| (*mint).clone()),
            kind: kinds
                .get(account.address.as_str())
                .copied()
                .unwrap_or(AssetKind::Unknown),
        })
        .collect()
}

/// Return a fresh route within the configured impact limit.
/// Retries API failures and expiry with bounded pacing; no-route and liquidity errors propagate
/// unchanged.
pub async fn fresh_route(
    provider: &impl SwapProvider,
    request: &SwapRequest,
    options: &CleanupOptions,
) -> Result<PreparedSwap> {
    let mut last = SwapError::Expired;
    for attempt in 0..options.quote_attempts.clamp(1, 3) {
        if !options.quote_interval.is_zero() {
            tokio::time::sleep(options.quote_interval).await;
        }
        match provider.build_swap(request).await.and_then(|fresh| {
            fresh.ensure_fresh(&options.swap_limits)?;
            fresh.quote.check_impact(&options.swap_limits)?;
            Ok(fresh)
        }) {
            Ok(fresh) => return Ok(fresh),
            Err(error @ (SwapError::Api(_) | SwapError::Expired)) => {
                last = error;
                if attempt + 1 < options.quote_attempts.clamp(1, 3)
                    && options.quote_interval.is_zero()
                {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }
            Err(error) => return Err(error),
        }
    }
    Err(last)
}

/// Build an exact-input swap request for the account balance and selected owner.
/// Returns `InvalidRequest` if the stored mint address cannot be parsed.
pub fn request(
    asset: &CleanupAsset,
    owner: &Pubkey,
    options: &CleanupOptions,
) -> Result<SwapRequest> {
    Ok(SwapRequest {
        mint: asset
            .account
            .mint
            .parse()
            .map_err(|_| SwapError::InvalidRequest("invalid mint".into()))?,
        wallet: *owner,
        raw_amount: asset.account.raw_amount,
        slippage_bps: options.slippage_bps,
    })
}

/// Return a read-only cleanup plan for the supplied backing accounts.
/// Protection and eligibility checks precede route lookup; only an explicit `NoRoute`
/// permits a burn classification. Provider failures remain unsupported, never burn fallbacks.
pub async fn build_plan_observed(
    wallet: &Pubkey,
    assets: Vec<CleanupAsset>,
    discovery_status: ScanStatus,
    unparsed_accounts: Vec<UnknownAsset>,
    provider: &impl SwapProvider,
    options: &CleanupOptions,
    observer: &dyn crate::core::progress::CleanupObserver,
) -> CleanupPlan {
    let mut plan = CleanupPlan {
        wallet: wallet.to_string(),
        discovery_status,
        entries: vec![],
        unparsed_accounts,
        summary: CleanupSummary::default(),
        selection: options.selection.clone(),
        policy: options.policy,
    };
    let total = assets.len();
    for (index, asset) in assets.into_iter().enumerate() {
        observer.progress(crate::core::progress::CleanupProgress {
            stage: "planning".into(),
            completed: index,
            total,
            operation: None,
            account: Some(asset.account.address.clone()),
            status: "running".into(),
        });
        let mut entry = CleanupEntry {
            asset,
            category: CleanupCategory::Unsupported,
            reason: String::new(),
            reason_code: CleanupReasonCode::ProviderFailure,
            quote: None,
        };

        // Apply user protection before balance checks or provider requests.
        if let Some(reason) = options.selection.skip_reason(&entry.asset.account) {
            entry.reason = reason.into();
            entry.reason_code = if options
                .selection
                .ignored_mints
                .contains(&entry.asset.account.mint)
            {
                CleanupReasonCode::IgnoredMint
            } else {
                CleanupReasonCode::NotSelected
            };
        } else if let Some((code, reason)) = eligibility_issue(&entry.asset, &plan.wallet) {
            entry.reason = reason;
            entry.reason_code = code;
        } else if entry.asset.account.raw_amount == 0 {
            entry.category = CleanupCategory::Empty;
            entry.reason = "Close empty account".into();
            entry.reason_code = CleanupReasonCode::EmptyAccount;
        } else if options.policy == CleanupPolicy::ExplicitDiscard {
            entry.category = CleanupCategory::Burnable;
            entry.reason_code = CleanupReasonCode::ExplicitDiscard;
            entry.reason =
                "Explicit discard: burn the approved full balance, then close; irreversible".into();
        } else {
            let result = match request(&entry.asset, wallet, options) {
                Ok(request) => fresh_route(provider, &request, options).await,
                Err(error) => Err(error),
            };

            // Choose swap or burn only from an explicit semantic route result.
            match result {
                Ok(_) if options.selection.protects_output() => {
                    entry.reason_code = CleanupReasonCode::IgnoredMint;
                    entry.reason = "protected WSOL may be unwrapped by a SOL swap; skipped".into();
                }
                Ok(fresh) => {
                    entry.category = CleanupCategory::Swappable;
                    entry.reason_code = CleanupReasonCode::SwapRoute;
                    entry.quote = Some(fresh.quote);
                    entry.reason = "swap full balance to native SOL, verify zero, close".into();
                }
                Err(SwapError::NoRoute(reason)) => {
                    entry.category = CleanupCategory::Burnable;
                    entry.reason_code = CleanupReasonCode::NoRoute;
                    entry.reason = format!("burn full balance then close; no route: {reason}");
                }
                Err(error) => {
                    entry.reason_code = match &error {
                        SwapError::UnsupportedNetwork(_) => CleanupReasonCode::RoutingUnavailable,
                        SwapError::InsufficientLiquidity(_) => {
                            CleanupReasonCode::InsufficientLiquidity
                        }
                        SwapError::PriceImpact { .. } => CleanupReasonCode::PriceImpact,
                        SwapError::Expired => CleanupReasonCode::QuoteExpired,
                        _ => CleanupReasonCode::ProviderFailure,
                    };
                    entry.reason =
                        format!("quote not safely actionable; no burn fallback: {error}");
                }
            }
        }
        plan.entries.push(entry);
    }

    // Compute preview totals from actionable entries and retain unparsed accounts as unsupported.
    let summary = &mut plan.summary;
    summary.token_accounts = plan.entries.len() + plan.unparsed_accounts.len();
    summary.unsupported = plan.unparsed_accounts.len();
    for entry in &plan.entries {
        match entry.category {
            CleanupCategory::Empty => summary.empty += 1,
            CleanupCategory::Swappable => {
                summary.swappable += 1;
                summary.estimated_swap_lamports += u128::from(
                    entry
                        .quote
                        .as_ref()
                        .expect("swappable quote")
                        .expected_out_lamports,
                );
            }
            CleanupCategory::Burnable => summary.burnable += 1,
            CleanupCategory::Unsupported => summary.unsupported += 1,
        }
        if entry.category != CleanupCategory::Unsupported {
            summary.accounts_to_close += 1;
            summary.estimated_reclaimed_lamports += u128::from(entry.asset.account.lamports);
        }
    }
    plan
}

/// CLI-compatible read-only planning without a host progress channel.
pub async fn build_plan(
    wallet: &Pubkey,
    assets: Vec<CleanupAsset>,
    discovery_status: ScanStatus,
    unparsed_accounts: Vec<UnknownAsset>,
    provider: &impl SwapProvider,
    options: &CleanupOptions,
) -> CleanupPlan {
    build_plan_observed(
        wallet,
        assets,
        discovery_status,
        unparsed_accounts,
        provider,
        options,
        &(),
    )
    .await
}
