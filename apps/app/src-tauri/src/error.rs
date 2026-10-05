use serde::Serialize;
use std::{collections::BTreeMap, fmt};

/// Stable codes for rejected IPC input and invalid backend configuration.
#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    InvalidRequest,
    InvalidWalletAddress,
    InvalidConfiguration,
}

/// Frontend-safe failure envelope; category read failures live in scanner statuses.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub code: ErrorCode,
    pub message: String,
    pub details: Option<BTreeMap<String, String>>,
}

impl AppError {
    pub(crate) fn invalid_request(reason: &str) -> Self {
        Self {
            code: ErrorCode::InvalidRequest,
            message: "Invalid wallet analysis request".into(),
            details: Some(BTreeMap::from([("reason".into(), concise_reason(reason))])),
        }
    }

    pub(crate) fn invalid_wallet() -> Self {
        Self {
            code: ErrorCode::InvalidWalletAddress,
            message: "walletAddress must be a valid Solana public key".into(),
            details: Some(BTreeMap::from([("field".into(), "walletAddress".into())])),
        }
    }

    pub(crate) fn configuration(field: &str, message: &str) -> Self {
        Self {
            code: ErrorCode::InvalidConfiguration,
            message: message.into(),
            details: Some(BTreeMap::from([("field".into(), field.into())])),
        }
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for AppError {}

/// Keep actionable scanner diagnostics without sending unbounded RPC responses to IPC.
pub(crate) fn concise_reason(reason: &str) -> String {
    let mut chars = reason.chars().filter(|character| !character.is_control());
    let mut message: String = chars.by_ref().take(512).collect();
    if chars.next().is_some() {
        message.push('…');
    }
    message
}
