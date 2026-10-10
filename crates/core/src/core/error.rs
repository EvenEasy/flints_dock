/// Distinguish route and validation failures from submission or confirmation uncertainty.
#[derive(Debug, thiserror::Error)]
pub enum SwapError {
    #[error("Invalid swap request: {0}")]
    InvalidRequest(String),
    #[error("No TOKEN/SOL route: {0}")]
    NoRoute(String),
    #[error("Insufficient liquidity: {0}")]
    InsufficientLiquidity(String),
    #[error("Insufficient funds: {0}")]
    InsufficientFunds(String),
    #[error("Routing unavailable on this network: {0}")]
    UnsupportedNetwork(String),
    #[error("Swap provider request failed: {0}")]
    Api(String),
    #[error("Invalid swap provider response: {0}")]
    InvalidResponse(String),
    #[error("Price impact {actual_pct:.4}% exceeds limit {max_pct:.4}%")]
    PriceImpact { actual_pct: f64, max_pct: f64 },
    #[error("Quote expired before submission; request a new preview")]
    Expired,
    #[error("Fresh quote is worse than the approved minimum; request a new preview")]
    PreviewChanged,
    #[error("RPC failed before submission: {0}")]
    Rpc(String),
    #[error("Simulation failed: {0}")]
    Simulation(String),
    #[error("Transaction failed on-chain ({signature}): {reason}")]
    TransactionFailed { signature: String, reason: String },
    #[error(
        "Submission/confirmation uncertain ({signature}): {reason}. Check this signature before retrying"
    )]
    Uncertain { signature: String, reason: String },
}

/// Use the shared semantic error type across swap and cleanup operations.
pub type Result<T> = std::result::Result<T, SwapError>;

use serde::Serialize;

/// Distinguish complete, partial, failed, unsupported and intentionally skipped discovery.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "status", content = "reason", rename_all = "snake_case")]
pub enum ScanStatus {
    Complete,
    Partial(String),
    Unsupported(String),
    Failed(String),
    Skipped(String),
}

impl ScanStatus {
    pub fn is_complete(&self) -> bool {
        matches!(self, Self::Complete)
    }
}
