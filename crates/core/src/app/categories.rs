//! Shared snapshot enrichment. Price, risk and route evidence remain independent.
use crate::{
    app::swap::{SwapError, SwapProvider, SwapRequest},
    core::categories::*,
    core::*,
};
use solana_pubkey::Pubkey;
use std::collections::{BTreeMap, BTreeSet};

/// Risk adapters return exact-mint observations and a separate coverage status.
pub trait RiskProvider {
    fn risks(
        &self,
        mints: &[String],
    ) -> impl std::future::Future<Output = (BTreeMap<String, RiskSignal>, ScanStatus)> + Send;
}

/// Mainnet-only routing is explicitly disabled for every other genesis hash.
pub struct ScopedSwap<'a, P> {
    pub provider: Option<&'a P>,
    pub mainnet: bool,
}
impl<P: SwapProvider + Sync> SwapProvider for ScopedSwap<'_, P> {
    async fn build_swap(
        &self,
        request: &SwapRequest,
    ) -> crate::app::swap::Result<crate::app::swap::PreparedSwap> {
        if !self.mainnet {
            return Err(SwapError::UnsupportedNetwork(
                "Jupiter supports mainnet only; routing unavailable on this network".into(),
            ));
        }
        self.provider
            .ok_or_else(|| SwapError::Api("Jupiter is unavailable".into()))?
            .build_swap(request)
            .await
    }
}

/// Produce categories from one inventory, without re-scanning token accounts per tile.
/// Dust means 0 < aggregate USD value <= the configured USD threshold; it says nothing about fees.
pub async fn classify<P: SwapProvider + RiskProvider + Sync>(
    snapshot: &WalletSnapshot,
    owner: &Pubkey,
    provider: Option<&P>,
    network: String,
    mainnet: bool,
    dust_threshold_usd: f64,
    compressed: CompressedReport,
) -> WalletCategories {
    let checked_at = now().to_string();
    let inventory_status = snapshot
        .scanners
        .get("all_tokens")
        .or(snapshot.scanners.get("tokens"))
        .cloned()
        .unwrap_or(ScanStatus::Unsupported(
            "Token inventory unavailable".into(),
        ));
    let holdings = crate::core::inventory::holdings(snapshot);
    let mints: Vec<_> = holdings
        .iter()
        .filter(|a| a.total_raw_amount > 0)
        .map(|a| a.mint.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let (risks, risk_status) = match (mainnet, provider) {
        (true, Some(provider)) => provider.risks(&mints).await,
        _ => (
            BTreeMap::new(),
            ScanStatus::Unsupported("Mainnet risk provider unavailable on this network".into()),
        ),
    };
    let mut scam = Vec::new();
    let mut dust = Vec::new();
    let mut dead = Vec::new();
    let mut route_failures = Vec::new();
    let scoped = ScopedSwap { provider, mainnet };
    for asset in holdings.iter().filter(|a| a.total_raw_amount > 0) {
        let mut item = CategoryItem {
            id: format!("{}:{}", asset.mint, asset.program.id()),
            mint: Some(asset.mint.clone()),
            program: Some(asset.program.id().to_string()),
            name: asset
                .metadata
                .name
                .clone()
                .or(asset.metadata.symbol.clone())
                .unwrap_or_else(|| asset.mint.clone()),
            kind: if asset.kind.is_fungible() {
                "fungible"
            } else if asset.kind.is_nft() {
                "nft"
            } else {
                "unknown"
            }
            .into(),
            accounts: asset.accounts.clone(),
            raw_amount: Some(asset.total_raw_amount.to_string()),
            risk: risks.get(&asset.mint).cloned(),
            valuation: if asset.value_usd.is_some() {
                "priced"
            } else {
                "unpriced"
            }
            .into(),
            value_usd: asset.value_usd,
            tradability: "unknown".into(),
            evidence: None,
            checked_at: checked_at.clone(),
            provider_scope: Some("Jupiter Swap V2 /build; mainnet; exact mint/raw amount".into()),
        };
        // Routing is independent of pricing. Do not truncate an aggregate larger than u64.
        if mainnet && asset.kind.is_fungible() {
            match (asset.mint.parse(), u64::try_from(asset.total_raw_amount)) {
                (Ok(mint), Ok(raw_amount)) if asset.mint != WRAPPED_SOL => {
                    match scoped
                        .build_swap(&SwapRequest {
                            mint,
                            raw_amount,
                            wallet: *owner,
                            slippage_bps: 50,
                        })
                        .await
                    {
                        Ok(_) => item.tradability = "route".into(),
                        Err(SwapError::NoRoute(evidence)) => {
                            item.tradability = "no_route".into();
                            item.checked_at = now().to_string();
                            item.evidence = Some(evidence);
                            dead.push(item.clone());
                        }
                        Err(error) => route_failures.push(error.to_string()),
                    }
                }
                _ => {
                    route_failures.push("Holding cannot be checked by token-to-SOL provider".into())
                }
            }
        }
        if item.risk.as_ref().is_some_and(|s| s.status == "suspicious") {
            scam.push(item.clone());
        }
        if asset.kind.is_fungible()
            && asset.value_usd.is_some_and(|value| {
                value.is_finite() && value > 0.0 && value <= dust_threshold_usd
            })
        {
            dust.push(item);
        }
    }
    let mut nfts = BTreeMap::new();
    let nft_item = |id: String, name: String, kind: &str, accounts: Vec<String>| CategoryItem {
        id: id.clone(),
        mint: (kind == "nft").then_some(id),
        program: None,
        name,
        kind: kind.into(),
        accounts,
        raw_amount: None,
        risk: None,
        valuation: "unpriced".into(),
        value_usd: None,
        tradability: "unknown".into(),
        evidence: Some("Owner-verified NFT standard evidence".into()),
        checked_at: checked_at.clone(),
        provider_scope: None,
    };
    for nft in &snapshot.classic_nfts {
        nfts.insert(
            nft.mint.clone(),
            nft_item(
                nft.mint.clone(),
                nft.metadata
                    .name
                    .clone()
                    .unwrap_or_else(|| nft.mint.clone()),
                "nft",
                nft.token_accounts.clone(),
            ),
        );
    }
    for core in &snapshot.core_assets {
        nfts.insert(
            core.address.clone(),
            nft_item(core.address.clone(), core.name.clone(), "core", vec![]),
        );
    }
    for cnft in compressed.items {
        nfts.entry(cnft.id.clone())
            .or_insert_with(|| nft_item(cnft.id, cnft.name, "compressed", vec![]));
    }
    let nft_status = combine_statuses(&[
        (
            "onchain",
            snapshot.scanners.get("nfts").unwrap_or(&inventory_status),
        ),
        ("DAS", &compressed.status),
    ]);
    let prices = snapshot
        .scanners
        .get("prices")
        .cloned()
        .unwrap_or(ScanStatus::Unsupported("Prices unavailable".into()));
    let routing = if !mainnet {
        ScanStatus::Unsupported("Jupiter routing is mainnet-only".into())
    } else if route_failures.is_empty() {
        inventory_status.clone()
    } else {
        ScanStatus::Partial(format!(
            "{} holdings have unknown routing: {}",
            route_failures.len(),
            route_failures[0]
        ))
    };
    let mut categories = BTreeMap::new();
    for (key, items, status) in [
        (
            "scam",
            scam,
            if matches!(risk_status, ScanStatus::Unsupported(_)) {
                risk_status.clone()
            } else {
                combine_statuses(&[("inventory", &inventory_status), ("risk", &risk_status)])
            },
        ),
        (
            "dust",
            dust,
            if matches!(prices, ScanStatus::Unsupported(_)) {
                prices.clone()
            } else {
                combine_statuses(&[("inventory", &inventory_status), ("pricing", &prices)])
            },
        ),
        ("dead_token", dead, routing.clone()),
        ("nft", nfts.into_values().collect(), nft_status),
    ] {
        categories.insert(
            key.into(),
            CategoryResult {
                items,
                status,
                checked_at: checked_at.clone(),
            },
        );
    }
    WalletCategories {
        network,
        dust_threshold_usd,
        categories,
        providers: BTreeMap::from([
            ("rpc".into(), inventory_status),
            ("pricing".into(), prices),
            ("risk".into(), risk_status),
            ("routing".into(), routing),
            ("das".into(), compressed.status),
            (
                "nft_classic".into(),
                snapshot
                    .scanners
                    .get("classic_nfts")
                    .cloned()
                    .unwrap_or(ScanStatus::Unsupported(
                        "Classic NFT check not requested".into(),
                    )),
            ),
            (
                "nft_core".into(),
                snapshot
                    .scanners
                    .get("core_asset_v1")
                    .cloned()
                    .unwrap_or(ScanStatus::Unsupported(
                        "Core NFT check not requested".into(),
                    )),
            ),
        ]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    struct Provider {
        requests: Mutex<Vec<SwapRequest>>,
        no_route: bool,
    }
    impl SwapProvider for Provider {
        async fn build_swap(
            &self,
            r: &SwapRequest,
        ) -> crate::app::swap::Result<crate::app::swap::PreparedSwap> {
            self.requests.lock().unwrap().push(r.clone());
            if self.no_route {
                Err(SwapError::NoRoute("exact provider evidence".into()))
            } else {
                Err(SwapError::Api("HTTP 429".into()))
            }
        }
    }
    impl RiskProvider for Provider {
        async fn risks(&self, mints: &[String]) -> (BTreeMap<String, RiskSignal>, ScanStatus) {
            (
                mints
                    .iter()
                    .map(|m| {
                        (
                            m.clone(),
                            RiskSignal {
                                status: "unknown".into(),
                                source: "fixture".into(),
                                reasons: vec![],
                                checked_at: "1".into(),
                            },
                        )
                    })
                    .collect(),
                ScanStatus::Partial("Unknown audits".into()),
            )
        }
    }
    fn snapshot() -> WalletSnapshot {
        WalletSnapshot {
            selected: ScanSelection {
                all_tokens: true,
                ..ScanSelection::ALL
            },
            owner: Pubkey::default().to_string(),
            commitment: "confirmed".into(),
            native_sol: None,
            token_accounts: vec![],
            tokens: vec![],
            all_tokens: vec![],
            mints: vec![],
            classic_nfts: vec![],
            core_assets: vec![],
            unknown_assets: vec![],
            scanners: BTreeMap::from([
                ("all_tokens".into(), ScanStatus::Complete),
                ("nfts".into(), ScanStatus::Complete),
                (
                    "prices".into(),
                    ScanStatus::Partial("Unpriced holdings".into()),
                ),
            ]),
            account_summary: crate::core::amount::summarize_accounts(&[]),
        }
    }
    fn token(n: u8, kind: AssetKind, value: Option<f64>) -> TokenAsset {
        TokenAsset {
            mint: Pubkey::new_from_array([n; 32]).to_string(),
            program: TokenProgram::Legacy,
            total_raw_amount: 100,
            decimals: Some(6),
            balance: Some("0.0001".into()),
            accounts: vec!["a".into(), "b".into()],
            kind,
            metadata: Default::default(),
            price: None,
            value_usd: value,
        }
    }
    #[tokio::test]
    async fn api_failure_is_unknown_unpriced_is_not_dust_and_nfts_never_route() {
        let mut wallet = snapshot();
        wallet.all_tokens = vec![
            token(1, AssetKind::Fungible, None),
            token(2, AssetKind::NonFungible, Some(0.001)),
            token(3, AssetKind::Fungible, Some(0.01)),
        ];
        let provider = Provider {
            requests: Default::default(),
            no_route: false,
        };
        let report = classify(
            &wallet,
            &Pubkey::default(),
            Some(&provider),
            "mainnet".into(),
            true,
            0.01,
            CompressedReport {
                items: vec![],
                status: ScanStatus::Unsupported("DAS missing".into()),
            },
        )
        .await;
        assert!(report.categories["dead_token"].items.is_empty());
        assert!(!report.categories["dead_token"].status.is_complete());
        assert_eq!(report.categories["dust"].items.len(), 1);
        assert_eq!(provider.requests.lock().unwrap().len(), 2);
        assert!(!report.categories["nft"].status.is_complete());
    }
    #[tokio::test]
    async fn no_route_evidence_counts_holdings_not_accounts_and_is_network_scoped() {
        let mut wallet = snapshot();
        let mut first = token(1, AssetKind::Fungible, None);
        first.accounts = vec!["a".into()];
        let mut second = first.clone();
        second.accounts = vec!["b".into()];
        wallet.all_tokens = vec![first, second];
        let provider = Provider {
            requests: Default::default(),
            no_route: true,
        };
        let report = classify(
            &wallet,
            &Pubkey::default(),
            Some(&provider),
            "mainnet".into(),
            true,
            0.01,
            CompressedReport {
                items: vec![],
                status: ScanStatus::Complete,
            },
        )
        .await;
        assert_eq!(report.categories["dead_token"].items.len(), 1);
        assert_eq!(report.categories["dead_token"].items[0].accounts.len(), 2);
        assert_eq!(
            report.categories["dead_token"].items[0]
                .raw_amount
                .as_deref(),
            Some("200")
        );
        assert_eq!(provider.requests.lock().unwrap()[0].raw_amount, 200);
        assert!(report.categories["dead_token"].items[0].evidence.is_some());
        let report = classify(
            &wallet,
            &Pubkey::default(),
            Some(&provider),
            "devnet".into(),
            false,
            0.01,
            CompressedReport {
                items: vec![],
                status: ScanStatus::Complete,
            },
        )
        .await;
        assert!(report.categories["dead_token"].items.is_empty());
        assert!(matches!(
            report.providers["routing"],
            ScanStatus::Unsupported(_)
        ));
        assert_eq!(provider.requests.lock().unwrap().len(), 1);
    }
    #[tokio::test]
    async fn devnet_test_observations_are_scoped_readonly_and_do_not_manufacture_nfts_or_swap_quotes()
     {
        use crate::app::test_observations::{DEVNET_GENESIS, TestManifest};
        let mut wallet = snapshot();
        let fungible = token(1, AssetKind::Fungible, None);
        let nft = token(2, AssetKind::NonFungible, None);
        wallet.all_tokens = vec![fungible.clone(), nft.clone()];
        let manifest = TestManifest::parse(&serde_json::json!({"network":DEVNET_GENESIS,"label":"read-only regression",
            "observations":[{"mint":fungible.mint,"program":fungible.program.id().to_string(),"suspicious":true,"unitUsd":0.001,"routing":"no_route"},
            {"mint":nft.mint,"program":nft.program.id().to_string(),"suspicious":true,"unitUsd":0.001,"routing":"no_route"}]}).to_string()).unwrap();
        let provider = Provider {
            requests: Default::default(),
            no_route: true,
        };
        let mut report = classify(
            &wallet,
            &Pubkey::default(),
            Some(&provider),
            DEVNET_GENESIS.into(),
            false,
            0.01,
            CompressedReport {
                items: vec![],
                status: ScanStatus::Unsupported("No DAS".into()),
            },
        )
        .await;
        manifest.apply(&mut report, &wallet);
        for key in ["scam", "dust", "dead_token"] {
            assert_eq!(report.categories[key].items.len(), 1);
            assert!(!report.categories[key].status.is_complete());
            assert!(
                report.categories[key].items[0]
                    .provider_scope
                    .as_ref()
                    .unwrap()
                    .starts_with("TEST DATA:")
            );
        }
        assert_eq!(
            report.categories["dead_token"].items[0].tradability,
            "test_no_route"
        );
        assert!(provider.requests.lock().unwrap().is_empty());

        // Semantic-token-only scans still expose configured observations without NFT inference.
        wallet.tokens = vec![fungible];
        wallet.all_tokens.clear();
        wallet.selected.all_tokens = false;
        manifest.apply(&mut report, &wallet);
        for key in ["scam", "dust", "dead_token"] {
            assert_eq!(report.categories[key].items.len(), 1);
        }
        let mut mainnet = classify(
            &wallet,
            &Pubkey::default(),
            None::<&Provider>,
            "mainnet".into(),
            true,
            0.01,
            CompressedReport {
                items: vec![],
                status: ScanStatus::Complete,
            },
        )
        .await;
        manifest.apply(&mut mainnet, &wallet);
        assert!(mainnet.categories["scam"].items.is_empty());
        assert!(!mainnet.providers.contains_key("test_data"));
    }

    #[tokio::test]
    async fn dust_uses_nonzero_aggregate_not_individual_accounts_or_unknown_prices() {
        let mut wallet = snapshot();
        let mut first = token(1, AssetKind::Fungible, Some(0.006));
        first.accounts = vec!["a".into()];
        let mut second = first.clone();
        second.accounts = vec!["b".into()];
        let mut empty = first.clone();
        empty.accounts = vec!["empty".into()];
        empty.total_raw_amount = 0;
        empty.value_usd = None;
        wallet.all_tokens = vec![first.clone(), second, empty.clone()];
        let report = classify::<Provider>(
            &wallet,
            &Pubkey::default(),
            None,
            "devnet".into(),
            false,
            0.01,
            CompressedReport {
                items: vec![],
                status: ScanStatus::Complete,
            },
        )
        .await;
        assert!(report.categories["dust"].items.is_empty());
        wallet.all_tokens = vec![first, empty];
        let report = classify::<Provider>(
            &wallet,
            &Pubkey::default(),
            None,
            "devnet".into(),
            false,
            0.01,
            CompressedReport {
                items: vec![],
                status: ScanStatus::Complete,
            },
        )
        .await;
        assert_eq!(report.categories["dust"].items.len(), 1);
        assert_eq!(report.categories["dust"].items[0].accounts.len(), 2);
    }
}
