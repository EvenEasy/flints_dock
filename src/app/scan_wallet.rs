use crate::{
    app::pricing::PriceProvider, core::amount::*, core::classification::classify_token_accounts,
    core::*,
};
use solana_pubkey::Pubkey;
use std::collections::BTreeMap;

/// Return a snapshot of the selected wallet categories, sharing token and metadata discovery.
/// The optional provider enriches USD prices only when enabled. Independent reads do not
/// form an atomic snapshot; failures are retained per scanner rather than discarding usable
/// results.
pub async fn scan_wallet<P: PriceProvider>(
    rpc: &impl WalletReader,
    owner: &Pubkey,
    options: &ScanOptions,
    provider: Option<&P>,
) -> WalletSnapshot {
    let selected = options.selection;

    // Run independent selected discovery paths concurrently and share their results.
    let (native, token_scan, core) = tokio::join!(
        async {
            if selected.balance {
                Some(rpc.native_balance(owner).await)
            } else {
                None
            }
        },
        async {
            if selected.needs_token_accounts() {
                Some(
                    rpc.token_inventory(owner, !(selected.tokens || selected.all_tokens))
                        .await,
                )
            } else {
                None
            }
        },
        async {
            if selected.nfts {
                Some(rpc.core_assets(owner).await)
            } else {
                None
            }
        },
    );
    let mut portfolio = WalletSnapshot {
        selected,
        owner: owner.to_string(),
        commitment: rpc.commitment(),
        native_sol: None,
        token_accounts: Vec::new(),
        tokens: Vec::new(),
        all_tokens: Vec::new(),
        mints: Vec::new(),
        classic_nfts: Vec::new(),
        core_assets: Vec::new(),
        unknown_assets: Vec::new(),
        scanners: BTreeMap::new(),
        account_summary: summarize_accounts(&[]),
    };
    if let Some(native) = native {
        match native {
            Ok(lamports) => {
                portfolio.native_sol = Some(NativeBalance {
                    lamports,
                    price: None,
                    value_usd: None,
                });
                portfolio
                    .scanners
                    .insert("native_sol".into(), ScanStatus::Complete);
            }
            Err(error) => {
                portfolio
                    .scanners
                    .insert("native_sol".into(), ScanStatus::Failed(error.to_string()));
            }
        }
    }

    // Classify the shared account inventory before projecting the requested asset views.
    if let Some(scan) = token_scan {
        let assets =
            classify_token_accounts(&scan.classification_accounts, &scan.mints, &scan.records);
        for asset in &assets {
            if asset.kind == AssetKind::Unknown || asset.metadata.name.is_none() {
                portfolio.unknown_assets.push(UnknownAsset {
                    address: asset.mint.clone(),
                    program_id: Some(asset.program.id().to_string()),
                    lamports: None,
                    reason: if asset.kind == AssetKind::Unknown {
                        "Unverified token classification"
                    } else {
                        "No recognized on-chain metadata"
                    }
                    .into(),
                    data: None,
                });
            }
        }
        if selected.tokens {
            portfolio.tokens = aggregate_tokens(&assets);
            portfolio
                .scanners
                .insert("tokens".into(), scan.status.clone());
        }
        if selected.all_tokens {
            portfolio.all_tokens = assets;
            portfolio
                .scanners
                .insert("all_tokens".into(), scan.status.clone());
        }
        if selected.nfts {
            portfolio.classic_nfts = scan
                .records
                .into_iter()
                .filter_map(|record| record.nft)
                .collect();
            portfolio
                .scanners
                .insert("classic_nfts".into(), scan.status);
        }
        portfolio.token_accounts = scan.accounts;
        portfolio.mints = scan.mints;
        portfolio.unknown_assets.extend(scan.unknown);
        portfolio.scanners.extend(scan.scanners);
        portfolio.account_summary = summarize_accounts(&portfolio.token_accounts);
    }

    // Combine classic and Core discovery while preserving partial NFT results.
    if let Some(core) = core {
        let classic = portfolio
            .scanners
            .get("classic_nfts")
            .cloned()
            .unwrap_or_else(|| ScanStatus::Failed("Classic NFT scanner did not run".into()));
        portfolio.scanners.insert(
            "nfts".into(),
            combine_statuses(&[("classic", &classic), ("core", &core.status)]),
        );
        portfolio
            .scanners
            .insert("core_asset_v1".into(), core.status);
        portfolio.core_assets = core.items;
        portfolio.unknown_assets.extend(core.unknown);
    }
    if selected.cnfts {
        portfolio
            .scanners
            .insert("compressed_nfts".into(), rpc.compressed_nfts(owner));
    }

    // Enrich priceable categories only after exact blockchain holdings are assembled.
    if selected.needs_prices() {
        let report =
            crate::app::pricing::price_portfolio(&mut portfolio, options.no_prices, provider).await;
        portfolio.scanners.insert("prices".into(), report);
    }
    portfolio
}

/// Read-only chain boundary. Implementations normalize external data and retain
/// partial failures; callers never interpret raw RPC envelopes or transport errors.
pub trait WalletReader {
    fn commitment(&self) -> String;

    /// Return the wallet lamport balance or a read failure.
    fn native_balance(
        &self,
        owner: &Pubkey,
    ) -> impl std::future::Future<Output = Result<u64, String>> + Send;

    /// Return decoded accounts and enrichment results, preserving partial failures.
    /// `nfts_only` limits mint and metadata work to potential NFT holdings.
    fn token_inventory(
        &self,
        owner: &Pubkey,
        nfts_only: bool,
    ) -> impl std::future::Future<Output = TokenInventory> + Send;

    /// Return wallet-owned Core assets with per-record decoding status.
    fn core_assets(
        &self,
        owner: &Pubkey,
    ) -> impl std::future::Future<Output = ScanCollection<CoreAsset>> + Send;

    /// Report cNFT discovery capability without substituting an unverified empty inventory.
    fn compressed_nfts(&self, owner: &Pubkey) -> ScanStatus;
}
