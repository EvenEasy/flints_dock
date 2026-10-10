//! Plan, then execute sequential account-scoped cleanup.
pub mod execute;
pub mod plan;
pub use crate::core::cleanup::*;
use crate::{app::swap::*, core::*};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use std::time::Duration;

/// Application-facing chain boundary. Planning uses only read-only preparation, never execution.
pub trait CleanupExecutor {
    /// Return current, enriched account state, or `None` if the account is absent.
    /// Unreadable state must be an error, not absence.
    fn refresh(
        &self,
        address: &str,
        owner: &Pubkey,
    ) -> impl std::future::Future<Output = Result<Option<CleanupAsset>>> + Send;

    /// Execute one operation and return its confirmed receipt.
    /// Swap requires matching fresh route data; each operation must revalidate the source
    /// and preserve a submitted signature when confirmation is uncertain.
    fn perform(
        &self,
        operation: CleanupOperation,
        asset: &CleanupAsset,
        fresh: Option<PreparedSwap>,
        signer: &Keypair,
        limits: &SwapLimits,
    ) -> impl std::future::Future<Output = Result<OperationReceipt>> + Send;

    /// Default mock/CLI boundary; RPC overrides this to journal before submission.
    fn perform_observed(
        &self,
        operation: CleanupOperation,
        asset: &CleanupAsset,
        fresh: Option<PreparedSwap>,
        signer: &Keypair,
        limits: &SwapLimits,
        _observer: &dyn crate::core::progress::CleanupObserver,
    ) -> impl std::future::Future<Output = Result<OperationReceipt>> + Send {
        self.perform(operation, asset, fresh, signer, limits)
    }

    /// Prepare a standard-aware NFT burn through read-only chain/provider checks.
    fn prepare_nft(
        &self,
        _target: &crate::core::nft_cleanup::NftTarget,
        _owner: &Pubkey,
    ) -> impl std::future::Future<Output = Result<crate::core::nft_cleanup::PreparedNftBurn>> + Send
    {
        async { Err(SwapError::InvalidRequest("NFT adapter unavailable".into())) }
    }

    /// Revalidate the saved NFT identity, then simulate/send/confirm without generic token burn.
    fn burn_nft(
        &self,
        _target: &crate::core::nft_cleanup::NftTarget,
        _approved: &crate::core::nft_cleanup::PreparedNftBurn,
        _signer: &Keypair,
        _limits: &SwapLimits,
        _observer: &dyn crate::core::progress::CleanupObserver,
    ) -> impl std::future::Future<Output = Result<OperationReceipt>> + Send {
        async { Err(SwapError::InvalidRequest("NFT adapter unavailable".into())) }
    }

    /// Retrieve metadata for a confirmed failed transaction, which can still charge fees.
    fn failed_receipt(
        &self,
        _operation: CleanupOperation,
        _signature: &str,
        _owner: &Pubkey,
    ) -> impl std::future::Future<Output = Option<OperationReceipt>> + Send {
        async { None }
    }
}

/// Discover backing accounts and return a read-only, account-scoped cleanup plan.
/// `options.selection` restricts accounts and protects mints. A requested account not found
/// in discovery is an error; partial discovery remains explicit in the returned plan.
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

    // Keep undecodable backing accounts visible without treating metadata-only audit records as
    // accounts.
    let unknown: Vec<_> = portfolio
        .unknown_assets
        .iter()
        .filter(|asset| {
            asset.data.is_some() && asset.lamports.is_some() && selected(&asset.address)
        })
        .cloned()
        .collect();

    // Fail an explicit selection if discovery cannot establish that the wallet owns it.
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

/// Complete wallet scenario, including standard-specific NFT preparation through the same executor.
/// The caller supplies only a network-verified compressed report; planning never receives a signer.
pub async fn plan_wallet_complete(
    reader: &impl crate::app::scan_wallet::WalletReader,
    owner: &Pubkey,
    provider: &impl SwapProvider,
    executor: &impl CleanupExecutor,
    options: &CleanupOptions,
    compressed: &crate::core::categories::CompressedReport,
) -> Result<CleanupPlan> {
    let snapshot = crate::app::scan_wallet::scan_wallet::<()>(
        reader,
        owner,
        &ScanOptions {
            selection: ScanSelection {
                all_tokens: true,
                ..ScanSelection::ALL
            },
            no_prices: true,
        },
        None,
    )
    .await;
    for address in &options.selection.accounts {
        if !snapshot
            .token_accounts
            .iter()
            .any(|a| &a.address == address)
            && !snapshot
                .unknown_assets
                .iter()
                .any(|a| &a.address == address)
        {
            return Err(SwapError::InvalidRequest(format!(
                "Requested account {address} was not discovered"
            )));
        }
    }
    let unparsed = snapshot
        .unknown_assets
        .iter()
        .filter(|a| {
            a.lamports.is_some()
                && !snapshot
                    .token_accounts
                    .iter()
                    .any(|t| t.address == a.address)
                && !snapshot.core_assets.iter().any(|c| c.address == a.address)
        })
        .cloned()
        .collect();
    let mut plan = plan::build_plan(
        owner,
        plan::assets_from_portfolio(&snapshot),
        snapshot
            .scanners
            .get("all_tokens")
            .cloned()
            .unwrap_or(ScanStatus::Failed("Discovery unavailable".into())),
        unparsed,
        provider,
        options,
    )
    .await;
    plan::add_nfts_observed(&mut plan, &snapshot, compressed, executor, options, &()).await;
    Ok(plan)
}
