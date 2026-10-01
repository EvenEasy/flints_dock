use super::*;
use crate::swap::{SwapError, SwapProvider, SwapRequest};

/// Conservative extension support: no confidential/withheld balances, transfer hooks,
/// CPI guards, pausing or other mutable extension semantics are guessed.
pub fn unsupported_reason(asset: &CleanupAsset, wallet: &str) -> Option<String> {
    let account = &asset.account;
    if account.owner != wallet {
        return Some("wallet is not token owner".into());
    }
    let mut empty = account.clone();
    empty.raw_amount = 0;
    match crate::scanner::tokens::assess_closure(&empty, wallet) {
        ClosureAssessment::PotentiallyReclaimable => {}
        ClosureAssessment::NotReclaimable(reason) | ClosureAssessment::NeedsReview(reason) => {
            return Some(reason);
        }
    }
    if !asset.kind.is_fungible() {
        return Some("NFT or unverified classification; cleanup excluded".into());
    }
    if account.decimals.is_none()
        || asset.mint.as_ref().is_none_or(|mint| {
            mint.mint != account.mint
                || mint.program != account.program
                || Some(mint.decimals) != account.decimals
        })
    {
        return Some("mint/decimals unavailable or inconsistent".into());
    }
    if asset.mint.as_ref().is_some_and(|mint| {
        mint.extension_types.iter().any(|ext| {
            !matches!(
                ext.as_str(),
                "MetadataPointer"
                    | "TokenMetadata"
                    | "MintCloseAuthority"
                    | "GroupPointer"
                    | "TokenGroup"
                    | "GroupMemberPointer"
                    | "TokenGroupMember"
            )
        })
    }) {
        return Some("unsupported mint extension".into());
    }
    if !matches!(account.state.as_str(), "Initialized" | "Frozen") {
        return Some("uninitialized account".into());
    }
    if account.raw_amount > 0 && account.state == "Frozen" {
        return Some("frozen balance cannot be swapped or burned".into());
    }
    if account.raw_amount > 0 && (account.is_native || account.mint == crate::jupiter::WRAPPED_SOL)
    {
        return Some("nonempty WSOL requires explicit unwrap; never burn native SOL".into());
    }
    None
}

pub fn assets_from_portfolio(portfolio: &Portfolio) -> Vec<CleanupAsset> {
    portfolio
        .token_accounts
        .iter()
        .map(|account| CleanupAsset {
            account: account.clone(),
            mint: portfolio
                .mints
                .iter()
                .find(|mint| mint.mint == account.mint && mint.program == account.program)
                .cloned(),
            kind: portfolio
                .all_tokens
                .iter()
                .find(|asset| asset.accounts.contains(&account.address))
                .map(|asset| asset.kind)
                .unwrap_or(AssetKind::Unknown),
        })
        .collect()
}

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

/// No keypair, executor, Price API or transaction submission is reachable from planning.
pub async fn build_plan(
    wallet: &Pubkey,
    assets: Vec<CleanupAsset>,
    discovery_status: ScanStatus,
    unparsed_accounts: Vec<UnknownAsset>,
    provider: &impl SwapProvider,
    options: &CleanupOptions,
) -> CleanupPlan {
    let mut plan = CleanupPlan {
        wallet: wallet.to_string(),
        discovery_status,
        entries: vec![],
        unparsed_accounts,
        summary: CleanupSummary::default(),
    };
    for asset in assets {
        let mut entry = CleanupEntry {
            asset,
            category: CleanupCategory::Unsupported,
            reason: String::new(),
            quote: None,
        };
        if let Some(reason) = unsupported_reason(&entry.asset, &plan.wallet) {
            entry.reason = reason;
        } else if entry.asset.account.raw_amount == 0 {
            entry.category = CleanupCategory::Empty;
            entry.reason = "close empty account".into();
        } else {
            let result = match request(&entry.asset, wallet, options) {
                Ok(request) => fresh_route(provider, &request, options).await,
                Err(error) => Err(error),
            };
            match result {
                Ok(fresh) => {
                    entry.category = CleanupCategory::Swappable;
                    entry.quote = Some(fresh.quote);
                    entry.reason = "swap full balance to native SOL, verify zero, close".into();
                }
                Err(SwapError::NoRoute(reason)) => {
                    entry.category = CleanupCategory::Burnable;
                    entry.reason = format!("burn full balance then close; no route: {reason}");
                }
                Err(error) => {
                    entry.reason =
                        format!("quote not safely actionable; no burn fallback: {error}");
                }
            }
        }
        plan.entries.push(entry);
    }
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
