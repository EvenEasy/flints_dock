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
    let unique: std::collections::BTreeMap<_, _> = portfolio
        .token_accounts
        .iter()
        .map(|a| (&a.address, a))
        .collect();
    unique
        .into_values()
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
/// Protection and eligibility checks precede route lookup; a genuine `NoRoute` or structurally unsupported swap capability
/// permits an approved burn. Provider failures remain unsupported, never burn fallbacks.
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
        nft_entries: vec![],
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
        } else if entry.asset.account.is_native {
            entry.category = CleanupCategory::Empty;
            entry.reason_code = CleanupReasonCode::NativeUnwrap;
            entry.reason = "Unwrap native-backed WSOL and close to the owner wallet".into();
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
                Err(SwapError::UnsupportedNetwork(reason))
                    if options.policy == CleanupPolicy::Complete =>
                {
                    entry.category = CleanupCategory::Burnable;
                    entry.reason_code = CleanupReasonCode::RoutingUnavailable;
                    entry.reason = format!(
                        "Burn the approved balance, then close; swap capability unavailable: {reason}"
                    );
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

/// Add standard-aware NFT actions to the same saved plan; unsupported targets remain visible.
/// Preparation is read-only and never grants execution authority.
pub async fn add_nfts_observed(
    plan: &mut CleanupPlan,
    snapshot: &WalletSnapshot,
    compressed: &crate::core::categories::CompressedReport,
    executor: &impl CleanupExecutor,
    options: &CleanupOptions,
    observer: &dyn crate::core::progress::CleanupObserver,
) {
    use crate::core::nft_cleanup::*;
    let mut targets = std::collections::BTreeMap::new();
    for entry in &plan.entries {
        if !entry.asset.kind.is_nft() || entry.asset.account.raw_amount == 0 {
            continue;
        }
        let target = NftTarget {
            id: entry.asset.account.mint.clone(),
            owner: plan.wallet.clone(),
            standard: NftStandard::TokenMetadata(entry.asset.kind),
            token_account: Some(entry.asset.account.address.clone()),
            mint: Some(entry.asset.account.mint.clone()),
        };
        targets.insert(target.id.clone(), target);
    }
    // Each confirmed classic NFT account is represented once by its standard-aware target.
    let removed = plan
        .entries
        .iter()
        .filter(|e| e.asset.kind.is_nft() && e.asset.account.raw_amount > 0)
        .count();
    plan.entries
        .retain(|e| !e.asset.kind.is_nft() || e.asset.account.raw_amount == 0);
    plan.summary.unsupported = plan.summary.unsupported.saturating_sub(removed);
    for core in &snapshot.core_assets {
        targets.insert(
            core.address.clone(),
            NftTarget {
                id: core.address.clone(),
                owner: core.owner.clone(),
                standard: NftStandard::Core,
                token_account: None,
                mint: None,
            },
        );
    }
    for cnft in &compressed.items {
        targets.entry(cnft.id.clone()).or_insert_with(|| NftTarget {
            id: cnft.id.clone(),
            owner: cnft.owner.clone(),
            standard: NftStandard::Compressed,
            token_account: None,
            mint: None,
        });
    }
    let owner: Pubkey = plan.wallet.parse().expect("validated plan owner");
    for (index, target) in targets.into_values().enumerate() {
        observer.progress(crate::core::progress::CleanupProgress {
            stage: "planning".into(),
            completed: index,
            total: snapshot.core_assets.len() + compressed.items.len() + removed,
            operation: Some(CleanupOperation::Burn),
            account: Some(target.id.clone()),
            status: "running".into(),
        });
        let (prepared, reason_code, reason) = if options.selection.skip_nft(&target) {
            (
                None,
                CleanupReasonCode::NotSelected,
                "Asset protected or not selected".into(),
            )
        } else {
            match executor.prepare_nft(&target, &owner).await {
                Ok(prepared) => (
                    Some(prepared),
                    CleanupReasonCode::NftBurn,
                    "Burn the selected NFT with its standard-specific instruction; irreversible"
                        .into(),
                ),
                Err(error) => (
                    None,
                    match &error {
                        SwapError::NftBlocked { code, .. } => *code,
                        SwapError::Rpc(_) | SwapError::Api(_) => CleanupReasonCode::ProviderFailure,
                        _ => CleanupReasonCode::NftEvidenceUnavailable,
                    },
                    error.to_string(),
                ),
            }
        };
        if prepared.is_none() {
            plan.summary.unsupported += 1;
        }
        plan.nft_entries.push(NftCleanupEntry {
            target,
            prepared,
            reason_code,
            reason,
        });
    }
    order_nft_dependencies(plan);
}

/// Keep masters alive for their selected prints; incomplete print ownership/selection blocks only the master.
fn order_nft_dependencies(plan: &mut CleanupPlan) {
    let mut prints = std::collections::BTreeMap::<String, u64>::new();
    for entry in &plan.nft_entries {
        if let Some(parent) = entry
            .prepared
            .as_ref()
            .and_then(|p| p.edition_parent.as_ref())
        {
            *prints.entry(parent.clone()).or_default() += 1;
        }
    }
    for entry in &mut plan.nft_entries {
        if let Some(prepared) = &entry.prepared
            && prepared.edition_count > 0
            && prints.get(&entry.target.id).copied().unwrap_or(0) != prepared.edition_count
        {
            entry.prepared = None;
            entry.reason_code = CleanupReasonCode::NftEvidenceUnavailable;
            entry.reason = "Master NFT has unselected, unowned or blocked print editions; keep it until those prints are burned".into();
            plan.summary.unsupported += 1;
        }
    }
    // Prints must consume the still-existing master token; master supply changes are verified again at send.
    plan.nft_entries.sort_by_key(|entry| {
        entry
            .prepared
            .as_ref()
            .is_none_or(|p| p.edition_parent.is_none())
    });
}
