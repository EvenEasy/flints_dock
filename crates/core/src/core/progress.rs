//! Host-independent progress and a durable-before-send transaction boundary.
use crate::core::cleanup::CleanupOperation;
use crate::core::error::SwapError;
use serde::Serialize;

/// An observed stage; counts describe completed accounts, never a timer estimate.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupProgress {
    pub stage: String,
    pub completed: usize,
    pub total: usize,
    pub operation: Option<CleanupOperation>,
    pub account: Option<String>,
    pub status: String,
}

/// Nonsensitive identity of a signed transaction, recorded before its first RPC submission.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingSubmission {
    pub wallet: String,
    pub account: String,
    pub mint: String,
    pub operation: CleanupOperation,
    pub signature: String,
    #[serde(serialize_with = "crate::core::integer_string")]
    pub expiry: u64,
}

/// Hosts may forward progress and persist signatures. A persistence failure prevents sending.
pub trait CleanupObserver: Send + Sync {
    fn progress(&self, _event: CleanupProgress) {}
    fn before_send(&self, _submission: PendingSubmission) -> Result<(), SwapError> {
        Ok(())
    }
    fn confirmed(&self, _signature: &str, _failed: bool) {}
}
impl CleanupObserver for () {}
