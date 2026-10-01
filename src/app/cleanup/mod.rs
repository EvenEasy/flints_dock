//! Plan, then execute sequential account-scoped cleanup.
pub mod execute;
pub mod plan;
pub use crate::core::cleanup::*;
use crate::{app::swap::*, core::*};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use std::time::Duration;
/// Application-facing chain boundary. Planning never receives this executor.
pub trait CleanupExecutor {
    fn refresh(
        &self,
        address: &str,
        owner: &Pubkey,
    ) -> impl std::future::Future<Output = Result<Option<CleanupAsset>>> + Send;
    fn perform(
        &self,
        operation: CleanupOperation,
        asset: &CleanupAsset,
        fresh: Option<PreparedSwap>,
        signer: &Keypair,
        limits: &SwapLimits,
    ) -> impl std::future::Future<Output = Result<OperationReceipt>> + Send;
}

/// Complete read-only scenario: discover backing accounts, restrict selection,
/// enrich/classify and quote. No signer or transaction executor is accepted.
pub async fn plan_wallet(
    reader: &impl crate::app::scan_wallet::WalletReader,
    owner: &Pubkey,
    provider: &impl SwapProvider,
    options: &CleanupOptions,
) -> Result<CleanupPlan> {
    let portfolio = crate::app::scan_wallet::scan_wallet::<()>(
        reader,
        owner,
        &ScanOptions {
            selection: ScanSelection {
                all_tokens: true,
                ..Default::default()
            },
            no_prices: true,
        },
        None,
    )
    .await;
    let selected = |address: &str| {
        options.selection.accounts.is_empty() || options.selection.accounts.contains(address)
    };
    let assets = plan::assets_from_portfolio(&portfolio)
        .into_iter()
        .filter(|asset| selected(&asset.account.address))
        .collect();
    let unknown: Vec<_> = portfolio
        .unknown_assets
        .iter()
        .filter(|asset| {
            asset.data.is_some() && asset.lamports.is_some() && selected(&asset.address)
        })
        .cloned()
        .collect();
    for address in &options.selection.accounts {
        if !portfolio
            .token_accounts
            .iter()
            .any(|account| &account.address == address)
            && !unknown.iter().any(|account| &account.address == address)
        {
            return Err(SwapError::InvalidRequest(format!(
                "requested account {address} was not discovered for this wallet"
            )));
        }
    }
    let status = portfolio
        .scanners
        .get("all_tokens")
        .cloned()
        .unwrap_or(ScanStatus::Failed("discovery did not run".into()));
    Ok(plan::build_plan(owner, assets, status, unknown, provider, options).await)
}
