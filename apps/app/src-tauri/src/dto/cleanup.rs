use dock_flints_core::core::{cleanup::*, progress::CleanupProgress};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Explicit IPC selection: NONE can never become the CLI's empty-allowlist ALL.
#[derive(Clone, Deserialize)]
#[serde(tag = "mode", rename_all = "camelCase", deny_unknown_fields)]
pub enum SelectionDto {
    All,
    Selected { mints: BTreeSet<String> },
    None,
}
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PrepareRequest {
    pub session_id: String,
    pub revision: u64,
    pub selection: SelectionDto,
    pub ignored_mints: BTreeSet<String>,
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
    pub account: String,
    pub mint: String,
    pub program: String,
    pub raw_amount: String,
    pub action: String,
    pub reason: String,
    pub expected_out_lamports: Option<String>,
    pub min_out_lamports: Option<String>,
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
