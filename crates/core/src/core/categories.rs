//! Evidence-based display categories; eligibility and transaction approval are separate.
use super::ScanStatus;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Unix seconds used for freshness. No secret/provider URL belongs in evidence.
pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// A provider flag is not proof of fraud. Missing audit fields are unknown.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RiskSignal {
    pub status: String,
    pub source: String,
    pub reasons: Vec<String>,
    pub checked_at: String,
}

/// A nonzero holding is evaluated once per mint/program, regardless of backing accounts.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryItem {
    pub id: String,
    pub mint: Option<String>,
    pub program: Option<String>,
    pub name: String,
    pub kind: String,
    pub accounts: Vec<String>,
    pub raw_amount: Option<String>,
    pub risk: Option<RiskSignal>,
    pub valuation: String,
    pub value_usd: Option<f64>,
    pub tradability: String,
    pub evidence: Option<String>,
    pub checked_at: String,
    pub provider_scope: Option<String>,
}

/// Count is only authoritative when status is complete; partial counts are observed lower bounds.
#[derive(Debug, Serialize)]
pub struct CategoryResult {
    pub items: Vec<CategoryItem>,
    pub status: ScanStatus,
    pub checked_at: String,
}
#[derive(Debug, Serialize)]
pub struct WalletCategories {
    pub network: String,
    pub dust_threshold_usd: f64,
    pub categories: BTreeMap<String, CategoryResult>,
    pub providers: BTreeMap<String, ScanStatus>,
}

/// Read-only indexed cNFT holding. Classic/Core IDs are deliberately excluded by the adapter.
#[derive(Debug, Clone, Serialize)]
pub struct CompressedAsset {
    pub id: String,
    pub owner: String,
    pub name: String,
}
#[derive(Debug)]
pub struct CompressedReport {
    pub items: Vec<CompressedAsset>,
    pub status: ScanStatus,
}
