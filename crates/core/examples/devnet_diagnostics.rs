//! Read-only snapshot-to-plan audit. Never loads a signer or submits a transaction.
use dock_flints_core::{
    app::{
        categories::{ScopedSwap, classify},
        cleanup::plan::{assets_from_portfolio, build_plan},
        scan_wallet::scan_wallet,
    },
    core::{cleanup::*, *},
    infra::{jupiter::Jupiter, solana},
};
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<_> = std::env::args().collect();
    anyhow::ensure!(args.len() == 3, "Use RPC PUBLIC_KEY");
    let owner = args[2].parse()?;
    let rpc = solana::client(args[1].clone(), 30);
    let network = rpc.get_genesis_hash().await?.to_string();
    anyhow::ensure!(
        network == dock_flints_core::app::test_observations::DEVNET_GENESIS,
        "This diagnostic requires verified devnet"
    );
    let snapshot = scan_wallet::<()>(
        &rpc,
        &owner,
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
    let provider = ScopedSwap::<Jupiter> {
        provider: None,
        mainnet: false,
    };
    let options = CleanupOptions {
        quote_interval: std::time::Duration::ZERO,
        ..Default::default()
    };
    // Retain token-program accounts whose bytes could not be decoded as visible skips.
    let unparsed: Vec<_> = snapshot
        .unknown_assets
        .iter()
        .filter(|asset| {
            asset.lamports.is_some()
                && asset.program_id.as_ref().is_some_and(|id| {
                    [TokenProgram::Legacy, TokenProgram::Token2022]
                        .iter()
                        .any(|program| program.id().to_string() == *id)
                })
        })
        .cloned()
        .collect();
    let auto = build_plan(
        &owner,
        assets_from_portfolio(&snapshot),
        snapshot.scanners["all_tokens"].clone(),
        unparsed.clone(),
        &provider,
        &options,
    )
    .await;
    let discard = build_plan(
        &owner,
        assets_from_portfolio(&snapshot),
        snapshot.scanners["all_tokens"].clone(),
        unparsed,
        &provider,
        &CleanupOptions {
            policy: CleanupPolicy::ExplicitDiscard,
            ..options
        },
    )
    .await;
    let categories = classify::<Jupiter>(
        &snapshot,
        &owner,
        None,
        network.clone(),
        false,
        0.01,
        dock_flints_core::core::categories::CompressedReport {
            items: vec![],
            status: ScanStatus::Unsupported("DAS endpoint not configured".into()),
        },
    )
    .await;
    println!(
        "{}",
        serde_json::to_string_pretty(
            &serde_json::json!({"network":network,"snapshot":{"owner":snapshot.owner,"tokenAccounts":snapshot.token_accounts,"mints":snapshot.mints,"allTokens":snapshot.all_tokens,"classicNfts":snapshot.classic_nfts,"coreAssets":snapshot.core_assets,"unknownAssets":snapshot.unknown_assets,"scanners":snapshot.scanners},"auto":auto,"explicitDiscardPreview":discard,"categories":categories,"transactionsSubmitted":0})
        )?
    );
    Ok(())
}
