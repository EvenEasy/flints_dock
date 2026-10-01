pub mod plan;
pub mod service;
use crate::{
    models::*,
    swap::{PreparedSwap, Result, SwapLimits, SwapQuote},
};
use serde::Serialize;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum CleanupCategory {
    Empty,
    Swappable,
    Burnable,
    Unsupported,
}
#[derive(Debug, Clone, Serialize)]
pub struct CleanupAsset {
    pub account: TokenAccount,
    pub mint: Option<MintInfo>,
    pub kind: AssetKind,
}
#[derive(Debug, Clone, Serialize)]
pub struct CleanupEntry {
    pub asset: CleanupAsset,
    pub category: CleanupCategory,
    pub reason: String,
    pub quote: Option<SwapQuote>,
}
#[derive(Debug, Serialize)]
pub struct CleanupPlan {
    pub wallet: String,
    pub discovery_status: ScanStatus,
    pub entries: Vec<CleanupEntry>,
    pub unparsed_accounts: Vec<UnknownAsset>,
    pub summary: CleanupSummary,
}
#[derive(Debug, Default, Serialize)]
pub struct CleanupSummary {
    pub token_accounts: usize,
    pub empty: usize,
    pub swappable: usize,
    pub burnable: usize,
    pub unsupported: usize,
    pub accounts_to_close: usize,
    #[serde(serialize_with = "integer_string")]
    pub estimated_swap_lamports: u128,
    #[serde(serialize_with = "integer_string")]
    pub estimated_reclaimed_lamports: u128,
}
#[derive(Debug, Clone)]
pub struct CleanupOptions {
    pub swap_limits: SwapLimits,
    pub slippage_bps: u16,
    pub quote_interval: Duration,
    pub quote_attempts: usize,
}
impl Default for CleanupOptions {
    fn default() -> Self {
        Self {
            swap_limits: SwapLimits::default(),
            slippage_bps: 50,
            quote_interval: Duration::from_millis(2100),
            quote_attempts: 2,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum CleanupOperation {
    Swap,
    Burn,
    Close,
}
#[derive(Debug, Clone, Serialize)]
pub struct OperationReceipt {
    pub operation: CleanupOperation,
    pub signature: String,
    #[serde(serialize_with = "optional_integer")]
    pub wallet_delta_lamports: Option<i128>,
    #[serde(serialize_with = "optional_integer")]
    pub reclaimed_lamports: Option<u64>,
}
fn optional_integer<T: ToString, S: serde::Serializer>(
    value: &Option<T>,
    serializer: S,
) -> std::result::Result<S::Ok, S::Error> {
    value
        .as_ref()
        .map(ToString::to_string)
        .serialize(serializer)
}
#[derive(Debug, Serialize)]
pub struct AccountCleanupResult {
    pub token_account: String,
    pub mint: String,
    pub category: CleanupCategory,
    pub status: String,
    pub reason: String,
    pub operations: Vec<OperationReceipt>,
    pub uncertain_signature: Option<String>,
}
#[derive(Debug, Default, Serialize)]
pub struct CleanupReport {
    pub results: Vec<AccountCleanupResult>,
    pub closed: usize,
    pub failed: usize,
    pub skipped: usize,
    #[serde(serialize_with = "integer_string")]
    pub known_swap_net_lamports: i128,
    #[serde(serialize_with = "integer_string")]
    pub known_reclaimed_lamports: u128,
    pub accounting_complete: bool,
}

/// Application-facing boundary; preview has no dependency on mutation methods.
pub trait CleanupExecutor {
    fn refresh(
        &self,
        address: &str,
        owner: &Pubkey,
    ) -> impl std::future::Future<Output = Result<Option<CleanupAsset>>> + Send;
    fn perform(
        &self,
        operation: CleanupOperation,
        asset: &CleanupAsset,
        fresh: Option<PreparedSwap>,
        signer: &Keypair,
        limits: &SwapLimits,
    ) -> impl std::future::Future<Output = Result<OperationReceipt>> + Send;
}
