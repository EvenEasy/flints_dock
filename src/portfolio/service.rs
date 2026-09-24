use crate::{
    models::*,
    portfolio::aggregate::*,
    pricing::{
        PriceProvider,
        jupiter::{Jupiter, WRAPPED_SOL},
    },
    scanner,
};
use anyhow::{Result, bail};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_pubkey::Pubkey;
use std::collections::BTreeMap;

pub async fn scan_wallet(
    rpc: &RpcClient,
    owner: &Pubkey,
    no_prices: bool,
    api_key: Option<String>,
    verbose: bool,
) -> Result<Portfolio> {
    if verbose {
        eprintln!(
            "Scanning SOL, both token programs, Core, stake authorities and nonce authorities..."
        );
    }
    let (native, mut legacy, mut token2022, core, stake, nonce) = tokio::join!(
        scanner::sol::get_solana_balance(rpc, owner),
        scanner::tokens::get_token_accounts(rpc, owner, TokenProgram::Legacy),
        scanner::tokens::get_token_accounts(rpc, owner, TokenProgram::Token2022),
        scanner::core::get_core_assets(rpc, owner),
        scanner::stake::get_stake_accounts(rpc, owner),
        scanner::accounts::get_nonce_accounts(rpc, owner),
    );
    if native.is_err()
        && [
            &legacy.status,
            &token2022.status,
            &core.status,
            &stake.status,
            &nonce.status,
        ]
        .iter()
        .all(|status| matches!(status, ScanStatus::Failed(_)))
    {
        bail!(
            "RPC scan failed for every on-chain scanner: {}",
            native
                .as_ref()
                .err()
                .map(ToString::to_string)
                .unwrap_or_default()
        );
    }
    let native_status = match &native {
        Ok(_) => ScanStatus::Complete,
        Err(error) => ScanStatus::Failed(error.to_string()),
    };
    let mut accounts = std::mem::take(&mut legacy.items);
    accounts.append(&mut token2022.items);
    accounts.sort_by(|a, b| a.address.cmp(&b.address));
    if verbose {
        eprintln!(
            "Found {} token accounts; fetching deduplicated mints and metadata in batches...",
            accounts.len()
        );
    }
    let mut mint_scan = scanner::metadata::get_mints(rpc, &mut accounts).await;
    let mut metadata_scan = scanner::nft::get_metadata(rpc, &accounts, &mint_scan.items).await;
    if !legacy.status.is_complete() || !token2022.status.is_complete() {
        mint_scan.issue(
            "Token discovery is incomplete; mint coverage is limited to discovered accounts",
        );
        metadata_scan
            .issue("Token discovery is incomplete; NFT coverage is limited to discovered accounts");
    }
    if !mint_scan.status.is_complete() {
        metadata_scan
            .issue("Mint enrichment is incomplete; some NFT candidates may not be verifiable");
    }
    let mut tokens = aggregate_tokens(&accounts, &mint_scan.items, &metadata_scan.items);
    let mut unknown = Vec::new();
    unknown.extend(legacy.unknown);
    unknown.extend(token2022.unknown);
    unknown.extend(core.unknown);
    unknown.extend(stake.unknown);
    unknown.extend(nonce.unknown);
    for token in &tokens {
        if token.kind == AssetKind::Unknown || token.metadata.name.is_none() {
            unknown.push(UnknownAsset {
                address: token.mint.clone(),
                program_id: Some(token.program.id().to_string()),
                lamports: None,
                reason: if token.kind == AssetKind::Unknown {
                    "Token retained with unverified asset classification"
                } else {
                    "Token retained without recognized on-chain metadata"
                }
                .into(),
                data: None,
            });
        }
    }
    let pricing = if no_prices {
        PriceReport {
            status: ScanStatus::Skipped("Disabled by --no-prices".into()),
            quotes: BTreeMap::new(),
        }
    } else if let Some(key) = api_key.filter(|key| !key.trim().is_empty()) {
        if verbose {
            eprintln!("Fetching optional Jupiter prices...");
        }
        let mut mints: Vec<_> = tokens
            .iter()
            .filter(|token| token.kind == AssetKind::Fungible && token.total_raw_amount > 0)
            .map(|token| token.mint.clone())
            .collect();
        mints.push(WRAPPED_SOL.into());
        match Jupiter::new(key) {
            Ok(provider) => provider.get_prices(&mints).await,
            Err(error) => PriceReport {
                status: ScanStatus::Failed(error.to_string()),
                quotes: BTreeMap::new(),
            },
        }
    } else {
        PriceReport {
            status: ScanStatus::Skipped(
                "Jupiter Price V3 requires JUPITER_API_KEY; balances remain available".into(),
            ),
            quotes: BTreeMap::new(),
        }
    };
    let sol_price = pricing
        .quotes
        .get(WRAPPED_SOL)
        .filter(|quote| quote.decimals == 9);
    let native_sol = native.ok().map(|lamports| NativeBalance {
        lamports,
        price: sol_price.cloned(),
        value_usd: sol_price
            .and_then(|price| approximate_value(u128::from(lamports), 9, price.usd)),
    });
    let mut pricing_status = pricing.status;
    for token in &mut tokens {
        let mint = mint_scan.items.iter().find(|mint| mint.mint == token.mint);
        // Scaled UI and interest-bearing units need provider-specific semantics.
        // Never silently price their unadjusted raw balance as an adjusted amount.
        let special_units = mint.is_some_and(|mint| {
            mint.extension_types
                .iter()
                .any(|kind| matches!(kind.as_str(), "ScaledUiAmount" | "InterestBearingConfig"))
        });
        if let Some(quote) = pricing.quotes.get(&token.mint)
            && token.kind == AssetKind::Fungible
        {
            if !special_units && token.decimals == Some(quote.decimals) {
                token.value_usd =
                    approximate_value(token.total_raw_amount, quote.decimals, quote.usd);
                token.price = Some(quote.clone());
            } else {
                pricing_status = ScanStatus::Partial("Some quotes excluded because decimals or extended UI unit semantics do not match".into());
            }
        }
    }
    // Account rent, stake-authority-only balances, and NFT rent are deliberately
    // excluded: they are not additional fungible holdings. Wrapped SOL is priced
    // as tokens once, never again as its token account lamport balance.
    let known_value_usd = sum_known_values(
        native_sol
            .iter()
            .map(|native| native.value_usd)
            .chain(tokens.iter().map(|token| token.value_usd)),
    );
    let account_summary = summarize_accounts(&accounts);
    let scanners = BTreeMap::from([
        ("native_sol".into(), native_status), ("legacy_tokens".into(), legacy.status),
        ("token_2022".into(), token2022.status), ("mint_metadata".into(), mint_scan.status),
        ("metaplex_metadata_and_nfts".into(), metadata_scan.status),
        ("core_asset_v1".into(), core.status),
        ("compressed_nfts".into(), scanner::cnft::support()),
        ("core_hashed_assets".into(), ScanStatus::Unsupported("HashedAssetV1 does not expose an enumerable owner field; only uncompressed AssetV1 is scanned".into())),
        ("stake_accounts".into(), stake.status), ("nonce_accounts".into(), nonce.status),
        ("other_accounts".into(), scanner::accounts::generic_support()), ("prices".into(), pricing_status),
    ]);
    Ok(Portfolio {
        owner: owner.to_string(), commitment: "confirmed (independent requests, not an atomic snapshot)".into(),
        native_sol, token_accounts: accounts, tokens, mints: mint_scan.items,
        classic_nfts: metadata_scan.items.into_iter().filter_map(|record| record.nft).collect(),
        core_assets: core.items, stake_accounts: stake.items, associated_accounts: nonce.items,
        unknown_assets: unknown, scanners, account_summary, known_value_usd,
        valuation_scope: "Approximate priced native SOL plus fungible token balances only. Excludes stake, nonce/account lamports, NFT/Core valuations and unpriced assets. This is not a complete portfolio valuation.".into(),
        limitations: vec![
            "RPC providers may disable or limit getProgramAccounts; each failed scanner is reported.".into(),
            "No off-chain metadata URIs are fetched; image URI is available only when embedded on-chain.".into(),
            "Core plugins/collection permissions, external Token-2022 metadata pointers and encrypted confidential balances are not fully interpreted.".into(),
            "Empty means public raw amount zero; extension-bearing accounts may still contain withheld/confidential funds. Reclaim estimates are conditional, before fees, and use actual lamports.".into(),
            "Direct wallet authorities only; delegates, multisigs, custodial holdings and protocol positions require additional interpretation.".into(),
        ],
    })
}
