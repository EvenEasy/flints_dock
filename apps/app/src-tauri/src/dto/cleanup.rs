use dock_flints_core::core::{cleanup::*, progress::CleanupProgress};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Explicit IPC selection: NONE can never become the CLI's empty-allowlist ALL.
#[derive(Clone, Deserialize)]
#[serde(
    tag = "mode",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum SelectionDto {
    All,
    Selected {
        #[serde(default)]
        mints: BTreeSet<String>,
        #[serde(default)]
        asset_ids: BTreeSet<String>,
    },
    None,
}
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PrepareRequest {
    pub session_id: String,
    pub revision: u64,
    pub selection: SelectionDto,
    pub ignored_mints: BTreeSet<String>,
    #[serde(default)]
    pub ignored_asset_ids: BTreeSet<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Approval {
    pub swap: bool,
    pub burn: bool,
    pub close: bool,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExecuteRequest {
    pub session_id: String,
    pub plan_id: String,
    pub approval: Approval,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct JobRequest {
    pub session_id: String,
    pub job_id: Option<String>,
    pub plan_id: Option<String>,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanEntryDto {
    pub asset_id: String,
    pub token_account: Option<String>,
    pub account: String,
    pub mint: String,
    pub program: String,
    pub raw_amount: String,
    pub action: String,
    pub reason: String,
    pub reason_code: CleanupReasonCode,
    pub decimals: Option<u8>,
    pub supply: Option<String>,
    pub kind: String,
    pub mint_authority: Option<String>,
    pub freeze_authority: Option<String>,
    pub close_authority: Option<String>,
    pub account_extensions: Vec<String>,
    pub mint_extensions: Vec<String>,
    pub expected_out_lamports: Option<String>,
    pub min_out_lamports: Option<String>,
}
/// Undecodable discovery stays visible but can never authorize a transaction.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UndecodableAccountDto {
    pub address: String,
    pub program: Option<String>,
    pub lamports: Option<String>,
    pub reason_code: CleanupReasonCode,
    pub reason: String,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupPlanDto {
    pub plan_id: String,
    pub session_id: String,
    pub revision: u64,
    pub network: String,
    pub expires_at: String,
    pub entries: Vec<PlanEntryDto>,
    pub selected_assets: usize,
    pub executable_accounts: usize,
    pub skipped_accounts: usize,
    pub undecodable_accounts: Vec<UndecodableAccountDto>,
    pub can_execute: bool,
    pub requires_burn: bool,
    pub swap_count: usize,
    pub burn_count: usize,
    pub close_count: usize,
    pub estimated_swap_lamports: String,
    pub estimated_reclaimed_lamports: String,
    pub max_price_impact_bps: u16,
    pub max_priority_fee_lamports: String,
    pub slippage_bps: u16,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressEvent {
    pub job_id: String,
    pub session_id: String,
    pub sequence: u64,
    #[serde(flatten)]
    pub progress: CleanupProgress,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupJobDto {
    pub job_id: String,
    pub session_id: String,
    pub network: String,
    pub status: String,
    pub sequence: u64,
    pub progress: Vec<ProgressEvent>,
    pub report: Option<CleanupReport>,
    pub error: Option<String>,
}
