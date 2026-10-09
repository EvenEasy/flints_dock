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
        Self {
            network: value.network,
            dust_threshold_usd: value.dust_threshold_usd,
            categories: value
                .categories
                .into_iter()
                .map(|(key, r)| {
                    (
                        key,
                        CategoryDto {
                            count: r.items.len(),
                            items: r.items,
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
