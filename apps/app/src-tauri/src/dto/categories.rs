use super::assets::ScanStatusDto;
use dock_flints_core::core::categories::{CategoryItem, WalletCategories};
use serde::Serialize;
use std::collections::BTreeMap;
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryDto {
    pub items: Vec<CategoryItem>,
    pub count: usize,
    pub status: ScanStatusDto,
    pub checked_at: String,
    pub source: String,
    pub network: String,
    pub reason: Option<String>,
    pub coverage: BTreeMap<String, ScanStatusDto>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoriesDto {
    pub network: String,
    pub dust_threshold_usd: f64,
    pub categories: BTreeMap<String, CategoryDto>,
    pub providers: BTreeMap<String, ScanStatusDto>,
}
impl From<WalletCategories> for CategoriesDto {
    fn from(value: WalletCategories) -> Self {
        let network = value.network.clone();
        let providers = value.providers.clone();
        Self {
            network: value.network,
            dust_threshold_usd: value.dust_threshold_usd,
            categories: value
                .categories
                .into_iter()
                .map(|(key, r)| {
                    (
                        key.clone(),
                        CategoryDto {
                            count: r.items.len(),
                            items: r.items,
                            reason: match &r.status {
                                dock_flints_core::core::ScanStatus::Complete => None,
                                dock_flints_core::core::ScanStatus::Partial(reason)
                                | dock_flints_core::core::ScanStatus::Unsupported(reason)
                                | dock_flints_core::core::ScanStatus::Failed(reason)
                                | dock_flints_core::core::ScanStatus::Skipped(reason) => {
                                    Some(reason.clone())
                                }
                            },
                            source: match key.as_str() {
                                "scam" => "Jupiter Tokens V2",
                                "dust" => "Jupiter Price V3",
                                "dead_token" => "Jupiter Swap V2",
                                _ => "Solana / Metaplex / MPL Core / DAS",
                            }
                            .into(),
                            network: network.clone(),
                            coverage: match key.as_str() {
                                "scam" => &["rpc", "risk"][..],
                                "dust" => &["rpc", "pricing"][..],
                                "dead_token" => &["rpc", "routing"][..],
                                "nft" => &["nft_classic", "nft_core", "das"][..],
                                _ => &[],
                            }
                            .iter()
                            .filter_map(|key| {
                                providers
                                    .get(*key)
                                    .cloned()
                                    .map(|s| ((*key).into(), s.into()))
                            })
                            .collect(),
                            status: r.status.into(),
                            checked_at: r.checked_at,
                        },
                    )
                })
                .collect(),
            providers: value
                .providers
                .into_iter()
                .map(|(k, s)| (k, s.into()))
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dock_flints_core::core::{ScanStatus, categories::CategoryResult};

    #[test]
    fn serialized_categories_keep_count_status_reason_and_dependency_coverage_separate() {
        let providers = BTreeMap::from([
            ("rpc".into(), ScanStatus::Complete),
            (
                "pricing".into(),
                ScanStatus::Skipped("Prices disabled".into()),
            ),
            ("risk".into(), ScanStatus::Failed("HTTP 401".into())),
            (
                "routing".into(),
                ScanStatus::Partial("Some routes unknown".into()),
            ),
            ("nft_classic".into(), ScanStatus::Complete),
            ("nft_core".into(), ScanStatus::Complete),
            (
                "das".into(),
                ScanStatus::Unsupported("DAS not configured".into()),
            ),
        ]);
        let categories = [
            ("scam", ScanStatus::Failed("HTTP 401".into())),
            ("dust", ScanStatus::Skipped("Prices disabled".into())),
            (
                "dead_token",
                ScanStatus::Partial("Some routes unknown".into()),
            ),
            ("nft", ScanStatus::Partial("DAS not configured".into())),
        ]
        .into_iter()
        .map(|(key, status)| {
            (
                key.into(),
                CategoryResult {
                    items: vec![],
                    status,
                    checked_at: "1".into(),
                },
            )
        })
        .collect();
        let dto: CategoriesDto = WalletCategories {
            network: "devnet-genesis".into(),
            dust_threshold_usd: 0.01,
            categories,
            providers,
        }
        .into();
        let json = serde_json::to_value(dto).unwrap();
        for (key, dependencies) in [
            ("scam", vec!["rpc", "risk"]),
            ("dust", vec!["rpc", "pricing"]),
            ("dead_token", vec!["rpc", "routing"]),
            ("nft", vec!["nft_classic", "nft_core", "das"]),
        ] {
            let category = &json["categories"][key];
            assert_eq!(category["count"], 0);
            assert_eq!(category["items"].as_array().unwrap().len(), 0);
            assert_eq!(category["checkedAt"], "1");
            assert!(category["reason"].is_string());
            assert!(category["status"]["status"].is_string());
            for dependency in dependencies {
                assert_eq!(
                    category["coverage"][dependency],
                    json["providers"][dependency]
                );
            }
        }
    }
}
