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
    let (risks, mut risk_status) = match (mainnet, provider) {
        (true, Some(provider)) => provider.risks(&mints).await,
        (true, None) => (
            BTreeMap::new(),
            ScanStatus::Failed("Mainnet risk provider is not configured".into()),
        ),
        (false, _) => (
            BTreeMap::new(),
            ScanStatus::Unsupported("Mainnet risk provider unavailable on this network".into()),
        ),
    };
    // A successful response is not complete risk coverage unless every requested
    // mint has an explicit observation and an attributable source.
    if risk_status.is_complete()
        && mints.iter().any(|mint| {
            risks.get(mint).is_none_or(|signal| {
                signal.source.trim().is_empty()
                    || !matches!(signal.status.as_str(), "suspicious" | "not_flagged")
            })
        })
    {
        risk_status = ScanStatus::Partial(
            "Some holdings have no explicit risk observation with a source".into(),
        );
    }
    let mut scam = Vec::new();
    let mut dust = Vec::new();
    let mut dead = Vec::new();
    let mut route_failures = Vec::new();
    let mut checked_routes = 0;
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
            provider_scope: None,
        };
        // Routing is independent of pricing. Do not truncate an aggregate larger than u64.
        if mainnet && provider.is_some() && asset.kind.is_fungible() && asset.mint != WRAPPED_SOL {
            match (asset.mint.parse(), u64::try_from(asset.total_raw_amount)) {
                (Ok(mint), Ok(raw_amount)) => {
                    item.provider_scope =
                        Some("Jupiter Swap V2 /build; mainnet; exact mint/raw amount".into());
                    match scoped
                        .build_swap(&SwapRequest {
                            mint,
                            raw_amount,
                            wallet: *owner,
                            slippage_bps: 50,
                        })
                        .await
                    {
                        Ok(_) => {
                            checked_routes += 1;
                            item.tradability = "route".into();
                        }
                        Err(SwapError::NoRoute(evidence)) => {
                            checked_routes += 1;
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
        } else if mainnet && provider.is_some() && asset.kind == AssetKind::Unknown {
            route_failures
                .push("Holding classification unresolved; routing coverage unknown".into());
        }
        if item
            .risk
            .as_ref()
            .is_some_and(|s| s.status == "suspicious" && !s.source.trim().is_empty())
        {
            let mut suspicious = item.clone();
            suspicious.provider_scope = suspicious
                .risk
                .as_ref()
                .map(|signal| format!("{}; exact mint; {network}", signal.source));
            scam.push(suspicious);
        }
        if asset.kind.is_fungible()
            && asset.value_usd.is_some_and(|value| {
                value.is_finite() && value > 0.0 && value <= dust_threshold_usd
            })
        {
            item.provider_scope = asset
                .price
                .as_ref()
                .map(|price| format!("{}; exact-mint valuation; {network}", price.source));
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
        let item = nfts.entry(nft.mint.clone()).or_insert_with(|| {
            let mut item = nft_item(
                nft.mint.clone(),
                nft.metadata
                    .name
                    .clone()
                    .unwrap_or_else(|| nft.mint.clone()),
                "nft",
                nft.token_accounts.clone(),
            );
            item.evidence = Some(nft.evidence.clone());
            item
        });
        item.accounts.extend(nft.token_accounts.iter().cloned());
        item.accounts.sort();
        item.accounts.dedup();
    }
    let owner = owner.to_string();
    for core in snapshot
        .core_assets
        .iter()
        .filter(|asset| asset.owner == owner)
    {
        nfts.insert(
            core.address.clone(),
            nft_item(core.address.clone(), core.name.clone(), "core", vec![]),
        );
    }
    for cnft in compressed
        .items
        .into_iter()
        .filter(|asset| asset.owner == owner)
    {
        nfts.entry(cnft.id.clone())
            .or_insert_with(|| nft_item(cnft.id, cnft.name, "compressed", vec![]));
    }
    let classic_status =
        snapshot
            .scanners
            .get("classic_nfts")
            .cloned()
            .unwrap_or(ScanStatus::Skipped(
                "Classic NFT check not requested".into(),
            ));
    let core_status = snapshot
        .scanners
        .get("core_asset_v1")
        .cloned()
        .unwrap_or(ScanStatus::Skipped("Core NFT check not requested".into()));
    let nft_status = combine_statuses(&[
        ("classic", &classic_status),
        ("Core", &core_status),
        ("DAS", &compressed.status),
    ]);
    let mut prices = snapshot
        .scanners
        .get("prices")
        .cloned()
        .unwrap_or(ScanStatus::Unsupported("Prices unavailable".into()));
    if prices.is_complete() {
        let unvalued = holdings
            .iter()
            .filter(|asset| {
                asset.total_raw_amount > 0
                    && (asset.kind == AssetKind::Unknown
                        || (asset.kind.is_fungible()
                            && asset
                                .value_usd
                                .is_none_or(|value| !value.is_finite() || value < 0.0)))
            })
            .count();
        if unvalued > 0 {
            prices = ScanStatus::Partial(format!(
                "{unvalued} holdings have unknown classification or no trustworthy valuation"
            ));
        }
    }
    let routing = if !mainnet {
        ScanStatus::Unsupported("Jupiter routing is mainnet-only".into())
    } else if provider.is_none() {
        ScanStatus::Failed("Mainnet routing provider is not configured".into())
    } else if route_failures.is_empty() {
        inventory_status.clone()
    } else {
        let reason = format!(
            "{} holdings have unknown routing: {}",
            route_failures.len(),
            route_failures[0]
        );
        let route_status = if checked_routes == 0 {
            ScanStatus::Failed(reason)
        } else {
            ScanStatus::Partial(reason)
        };
        combine_statuses(&[("inventory", &inventory_status), ("routing", &route_status)])
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
            ("nft_classic".into(), classic_status),
            ("nft_core".into(), core_status),
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
    struct EvidenceProvider {
        source: String,
    }
    impl SwapProvider for EvidenceProvider {
        async fn build_swap(
            &self,
            _: &SwapRequest,
        ) -> crate::app::swap::Result<crate::app::swap::PreparedSwap> {
            Err(SwapError::NoRoute(
                "Exact mint, amount and network route lookup returned NoRoute".into(),
            ))
        }
    }
    impl RiskProvider for EvidenceProvider {
        async fn risks(&self, mints: &[String]) -> (BTreeMap<String, RiskSignal>, ScanStatus) {
            (
                mints
                    .iter()
                    .enumerate()
                    .map(|(index, mint)| {
                        (
                            mint.clone(),
                            RiskSignal {
                                status: if index == 0 {
                                    "suspicious"
                                } else {
                                    "not_flagged"
                                }
                                .into(),
                                source: self.source.clone(),
                                reasons: vec!["Explicit exact-mint provider observation".into()],
                                checked_at: "1".into(),
                            },
                        )
                    })
                    .collect(),
                ScanStatus::Complete,
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
                ("classic_nfts".into(), ScanStatus::Complete),
                ("core_asset_v1".into(), ScanStatus::Complete),
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
    fn classic(n: u8, account: &str) -> NftAsset {
        NftAsset {
            mint: Pubkey::new_from_array([n; 32]).to_string(),
            token_accounts: vec![account.into()],
            metadata: Default::default(),
            programmable: false,
            edition: false,
            evidence: "Verified Metaplex NonFungible owner account".into(),
        }
    }
    fn account(address: &str, mint: &str) -> TokenAccount {
        TokenAccount {
            address: address.into(),
            mint: mint.into(),
            program: TokenProgram::Legacy,
            program_id: TokenProgram::Legacy.id().to_string(),
            owner: Pubkey::default().to_string(),
            raw_amount: 100,
            decimals: Some(6),
            lamports: 1,
            data_len: 165,
            state: "initialized".into(),
            delegate: None,
            delegated_amount: 0,
            close_authority: None,
            is_native: false,
            native_reserve_lamports: None,
            extensions: serde_json::json!({}),
            extension_types: vec![],
            closure: ClosureAssessment::NeedsReview("Nonzero holding".into()),
        }
    }
    fn core(n: u8, owner: &str) -> CoreAsset {
        CoreAsset {
            address: Pubkey::new_from_array([n; 32]).to_string(),
            owner: owner.into(),
            name: "Core asset".into(),
            uri: String::new(),
            lamports: 1,
            data_len: 1,
            update_authority: String::new(),
            collection: None,
            plugins_status: ScanStatus::Complete,
        }
    }

    #[tokio::test]
    async fn nft_ids_are_deduplicated_and_known_classic_core_survive_missing_das() {
        let mut wallet = snapshot();
        wallet.classic_nfts = vec![classic(7, "a"), classic(7, "b"), classic(7, "a")];
        wallet.core_assets = vec![
            core(8, &wallet.owner),
            core(8, &wallet.owner),
            core(9, "foreign"),
        ];
        wallet.scanners.insert(
            "prices".into(),
            ScanStatus::Failed("Pricing HTTP 401".into()),
        );
        let report = classify::<Provider>(
            &wallet,
            &Pubkey::default(),
            None,
            "devnet".into(),
            false,
            0.01,
            CompressedReport {
                items: vec![],
                status: ScanStatus::Unsupported("DAS not configured".into()),
            },
        )
        .await;
        let nft = &report.categories["nft"];
        assert_eq!(nft.items.len(), 2);
        assert!(
            matches!(&nft.status, ScanStatus::Partial(reason) if reason.contains("DAS not configured"))
        );
        let classic = nft.items.iter().find(|item| item.kind == "nft").unwrap();
        assert_eq!(classic.accounts, ["a", "b"]);
        assert_eq!(
            classic.evidence.as_deref(),
            Some("Verified Metaplex NonFungible owner account")
        );
        assert!(report.categories["dust"].items.is_empty());
    }

    #[tokio::test]
    async fn compressed_ids_are_unique_and_foreign_ownership_is_excluded() {
        let wallet = snapshot();
        let asset = CompressedAsset {
            id: "compressed-id".into(),
            owner: wallet.owner.clone(),
            name: "cNFT".into(),
        };
        let report = classify::<Provider>(
            &wallet,
            &Pubkey::default(),
            None,
            "devnet".into(),
            false,
            0.01,
            CompressedReport {
                items: vec![
                    asset.clone(),
                    asset,
                    CompressedAsset {
                        id: "foreign-id".into(),
                        owner: "foreign".into(),
                        name: "foreign".into(),
                    },
                ],
                status: ScanStatus::Complete,
            },
        )
        .await;
        assert_eq!(report.categories["nft"].items.len(), 1);
        assert!(report.categories["nft"].status.is_complete());
    }

    #[tokio::test]
    async fn empty_onchain_nfts_with_unknown_compressed_coverage_do_not_assert_complete_zero() {
        let mut wallet = snapshot();
        let report = classify::<Provider>(
            &wallet,
            &Pubkey::default(),
            None,
            "devnet".into(),
            false,
            0.01,
            CompressedReport {
                items: vec![],
                status: ScanStatus::Unsupported("No DAS".into()),
            },
        )
        .await;
        assert!(report.categories["nft"].items.is_empty());
        assert!(matches!(
            report.categories["nft"].status,
            ScanStatus::Partial(_)
        ));
        // Complete token inventory does not establish that NFT scanners ran.
        wallet.scanners.remove("classic_nfts");
        wallet.scanners.remove("core_asset_v1");
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
        assert!(!report.categories["nft"].status.is_complete());
        assert!(matches!(
            report.providers["nft_classic"],
            ScanStatus::Skipped(_)
        ));
    }

    #[tokio::test]
    async fn mainnet_missing_provider_does_not_produce_authoritative_risk_or_routing_zero() {
        let wallet = snapshot();
        let report = classify::<Provider>(
            &wallet,
            &Pubkey::default(),
            None,
            "mainnet".into(),
            true,
            0.01,
            CompressedReport {
                items: vec![],
                status: ScanStatus::Complete,
            },
        )
        .await;
        assert!(matches!(report.providers["risk"], ScanStatus::Failed(_)));
        assert!(matches!(report.providers["routing"], ScanStatus::Failed(_)));
        assert!(!report.categories["scam"].status.is_complete());
        assert!(!report.categories["dead_token"].status.is_complete());
    }

    #[tokio::test]
    async fn complete_categories_retain_distinct_nonzero_items_from_independent_evidence() {
        let mut wallet = snapshot();
        wallet
            .scanners
            .insert("prices".into(), ScanStatus::Complete);
        wallet.all_tokens = vec![
            token(1, AssetKind::Fungible, Some(0.001)),
            token(2, AssetKind::Fungible, Some(0.002)),
            token(3, AssetKind::Fungible, Some(0.003)),
            token(4, AssetKind::Fungible, Some(1.0)),
        ];
        wallet.classic_nfts = vec![classic(7, "nft-account")];
        wallet.core_assets = vec![core(8, &wallet.owner)];
        let provider = EvidenceProvider {
            source: "Exact-mint risk provider".into(),
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
        for (key, count) in [("scam", 1), ("nft", 2), ("dust", 3), ("dead_token", 4)] {
            assert!(report.categories[key].status.is_complete(), "{key}");
            assert_eq!(report.categories[key].items.len(), count, "{key}");
        }
        assert!(
            report.categories["scam"].items[0]
                .risk
                .as_ref()
                .unwrap()
                .source
                .contains("risk provider")
        );
        assert!(
            report.categories["dead_token"]
                .items
                .iter()
                .all(|item| item.tradability == "no_route" && item.evidence.is_some())
        );
    }

    #[tokio::test]
    async fn missing_risk_source_and_missing_valuation_remain_unknown_with_complete_provider_status()
     {
        let mut wallet = snapshot();
        wallet
            .scanners
            .insert("prices".into(), ScanStatus::Complete);
        wallet.all_tokens = vec![token(1, AssetKind::Fungible, None)];
        let provider = EvidenceProvider { source: " ".into() };
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
        assert!(report.categories["scam"].items.is_empty());
        assert!(matches!(
            report.categories["scam"].status,
            ScanStatus::Partial(_)
        ));
        assert!(report.categories["dust"].items.is_empty());
        assert!(matches!(
            report.categories["dust"].status,
            ScanStatus::Partial(_)
        ));
        assert_eq!(report.categories["dead_token"].items.len(), 1);
        assert!(report.categories["dead_token"].status.is_complete());
        assert!(report.categories["nft"].status.is_complete());
        assert!(report.categories["nft"].items.is_empty());
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
    async fn duplicate_raw_accounts_and_overlapping_views_do_not_double_count_holdings() {
        let mut wallet = snapshot();
        wallet
            .scanners
            .insert("prices".into(), ScanStatus::Complete);
        let mut holding = token(1, AssetKind::Fungible, Some(0.01));
        holding.price = Some(Price {
            usd: 50.0,
            source: "Exact-mint quote".into(),
            block_id: Some(1),
            decimals: 6,
        });
        let a = account("a", &holding.mint);
        let b = account("b", &holding.mint);
        wallet.token_accounts = vec![a.clone(), a, b.clone(), b];
        wallet.all_tokens = vec![holding.clone(), holding.clone()];
        wallet.tokens = vec![holding];
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
        for key in ["dust", "dead_token"] {
            assert_eq!(report.categories[key].items.len(), 1);
            assert_eq!(report.categories[key].items[0].accounts, ["a", "b"]);
            assert_eq!(
                report.categories[key].items[0].raw_amount.as_deref(),
                Some("200")
            );
            assert_eq!(report.categories[key].items[0].value_usd, Some(0.01));
        }
        assert_eq!(provider.requests.lock().unwrap().len(), 1);
        assert_eq!(provider.requests.lock().unwrap()[0].raw_amount, 200);
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
