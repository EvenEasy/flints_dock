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
                            source: if key != "nft" && providers.contains_key("test_data") {
                                "TEST DATA: devnet manifest"
                            } else {
                                match key.as_str() {
                                    "scam" => "Jupiter Tokens V2",
                                    "dust" => "Jupiter Price V3",
                                    "dead_token" => "Jupiter Swap V2",
                                    _ => "Solana / Metaplex / MPL Core / DAS",
                                }
                            }
                            .into(),
                            network: network.clone(),
                            coverage: if key == "nft" {
                                ["nft_classic", "nft_core", "das"]
                                    .into_iter()
                                    .filter_map(|key| {
                                        providers.get(key).cloned().map(|s| (key.into(), s.into()))
                                    })
                                    .collect()
                            } else {
                                BTreeMap::new()
                            },
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
