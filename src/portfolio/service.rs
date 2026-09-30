use crate::{
    classification::classify_token_accounts,
    models::*,
    portfolio::aggregate::*,
    pricing::{PriceProvider, jupiter::Jupiter},
    scanner,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_pubkey::Pubkey;
use std::collections::BTreeMap;

/// CLI entry point. Construct the optional HTTP provider only for selected priceable categories.
pub async fn scan_wallet(
    rpc: &RpcClient,
    owner: &Pubkey,
    options: &ScanOptions,
    api_key: Option<String>,
) -> Portfolio {
    let provider = if options.selection.needs_prices() && !options.no_prices {
        api_key
            .filter(|key| !key.trim().is_empty())
            .map(Jupiter::new)
            .transpose()
    } else {
        Ok(None)
    };
    let mut portfolio = scan_wallet_with_provider(
        rpc,
        owner,
        options,
        provider.as_ref().ok().and_then(Option::as_ref),
    )
    .await;
    if let Err(error) = provider {
        portfolio
            .scanners
            .insert("prices".into(), ScanStatus::Failed(error.to_string()));
    }
    if options.verbose {
        for (name, status) in &portfolio.scanners {
            eprintln!("{name}: {status:?}");
        }
    }
    portfolio
}

/// Injecting a provider lets tests verify that excluded categories make no pricing calls.
pub async fn scan_wallet_with_provider<P: PriceProvider>(
    rpc: &RpcClient,
    owner: &Pubkey,
    options: &ScanOptions,
    provider: Option<&P>,
) -> Portfolio {
    let selected = options.selection;
    if options.verbose {
        eprintln!("Selected categories: {selected:?}");
    }
    let (native, token_scan, core) = tokio::join!(
        async {
            if selected.balance {
                Some(scanner::sol::get_solana_balance(rpc, owner).await)
            } else {
                None
            }
        },
        async {
            if selected.needs_token_accounts() {
                Some(
                    scan_tokens(
                        rpc,
                        owner,
                        !(selected.tokens || selected.all_tokens),
                        options.verbose,
                    )
                    .await,
                )
            } else {
                None
            }
        },
        async {
            if selected.nfts {
                Some(scanner::core::get_core_assets(rpc, owner).await)
            } else {
                None
            }
        },
    );
    let mut portfolio = Portfolio {
        selected,
        owner: owner.to_string(),
        commitment: format!("{:?} (independent requests)", rpc.commitment().commitment),
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
        portfolio.scanners.insert(
            "compressed_nfts".into(),
            scanner::cnft::get_compressed_nfts(owner),
        );
    }
    if selected.needs_prices() {
        let report =
            crate::pricing::price_portfolio(&mut portfolio, options.no_prices, provider).await;
        portfolio.scanners.insert("prices".into(), report);
    }
    portfolio
}

struct TokenScan {
    accounts: Vec<TokenAccount>,
    classification_accounts: Vec<TokenAccount>,
    mints: Vec<MintInfo>,
    records: Vec<MetadataRecord>,
    unknown: Vec<UnknownAsset>,
    status: ScanStatus,
    scanners: BTreeMap<String, ScanStatus>,
}

async fn scan_tokens(rpc: &RpcClient, owner: &Pubkey, nfts_only: bool, verbose: bool) -> TokenScan {
    let (mut legacy, mut token2022) = tokio::join!(
        scanner::tokens::get_token_accounts(rpc, owner, TokenProgram::Legacy),
        scanner::tokens::get_token_accounts(rpc, owner, TokenProgram::Token2022),
    );
    let discovery = combine_statuses(&[
        ("legacy", &legacy.status),
        ("token_2022", &token2022.status),
    ]);
    let mut accounts = std::mem::take(&mut legacy.items);
    accounts.append(&mut token2022.items);
    accounts.sort_by(|a, b| a.address.cmp(&b.address));
    // NFT-only runs need mint/metadata lookups only for possible NFT holdings.
    // Keep all raw accounts internally, without guessing decimals for unqueried mints.
    let mut candidates: Vec<_> = accounts
        .iter()
        .filter(|account| !nfts_only || account.raw_amount == 1)
        .cloned()
        .collect();
    let mint_scan = scanner::metadata::get_mints(rpc, &mut candidates).await;
    for account in &mut accounts {
        account.decimals = mint_scan
            .items
            .iter()
            .find(|mint| mint.mint == account.mint && mint.program == account.program)
            .map(|mint| mint.decimals);
    }
    if nfts_only {
        candidates.retain(|account| {
            account.decimals == Some(0)
                && mint_scan
                    .items
                    .iter()
                    .any(|mint| mint.mint == account.mint && mint.supply == 1)
        });
    }
    if verbose {
        eprintln!(
            "{} token accounts; {} accounts require metadata classification",
            accounts.len(),
            candidates.len()
        );
    }
    let metadata_scan = scanner::nft::get_metadata(rpc, &candidates, &mint_scan.items).await;
    let status = if matches!(discovery, ScanStatus::Failed(_)) {
        discovery.clone()
    } else {
        combine_statuses(&[
            ("discovery", &discovery),
            ("mints", &mint_scan.status),
            ("metadata", &metadata_scan.status),
        ])
    };
    let scanners = BTreeMap::from([
        ("legacy_tokens".into(), legacy.status),
        ("token_2022".into(), token2022.status),
        ("mint_metadata".into(), mint_scan.status),
        ("metaplex_metadata_and_nfts".into(), metadata_scan.status),
    ]);
    let mut unknown = legacy.unknown;
    unknown.extend(token2022.unknown);
    TokenScan {
        accounts,
        classification_accounts: candidates,
        mints: mint_scan.items,
        records: metadata_scan.items,
        unknown,
        status,
        scanners,
    }
}

pub fn combine_statuses(statuses: &[(&str, &ScanStatus)]) -> ScanStatus {
    if statuses.iter().all(|(_, status)| status.is_complete()) {
        return ScanStatus::Complete;
    }
    let reason = statuses
        .iter()
        .filter(|(_, status)| !status.is_complete())
        .map(|(name, status)| {
            format!(
                "{name}: {}",
                match status {
                    ScanStatus::Partial(reason)
                    | ScanStatus::Failed(reason)
                    | ScanStatus::Unsupported(reason)
                    | ScanStatus::Skipped(reason) => reason,
                    ScanStatus::Complete => "",
                }
            )
        })
        .collect::<Vec<_>>()
        .join("; ");
    if statuses.iter().all(|(_, status)| {
        matches!(
            status,
            ScanStatus::Failed(_) | ScanStatus::Unsupported(_) | ScanStatus::Skipped(_)
        )
    }) {
        ScanStatus::Failed(reason)
    } else {
        ScanStatus::Partial(reason)
    }
}
