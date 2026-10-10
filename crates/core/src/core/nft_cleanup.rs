//! Standard-aware NFT burn targets and server-owned preparation; no transport or UI dependencies.
use crate::core::{AssetKind, cleanup::CleanupReasonCode};
use serde::Serialize;
use solana_instruction::Instruction;

/// NFT identities are asset IDs; Core/cNFT have no SPL source account.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum NftStandard {
    TokenMetadata(AssetKind),
    Core,
    Compressed,
}

/// Bind the exact owned asset, standard and optional SPL backing account to approval.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NftTarget {
    pub id: String,
    pub owner: String,
    pub standard: NftStandard,
    pub token_account: Option<String>,
    pub mint: Option<String>,
}

/// Instructions stay in Rust; immutable identity is compared again before submission.
#[derive(Debug, Clone)]
pub struct PreparedNftBurn {
    pub identity: serde_json::Value,
    pub instructions: Vec<Instruction>,
    /// Print editions depend on their master token remaining available until the prints are burned.
    pub edition_parent: Option<String>,
    /// Mutable master print count is checked as a dependency, not an immutable identity field.
    pub edition_count: u64,
}

/// Retain unsupported NFT targets and their precise reasons alongside executable burns.
#[derive(Debug, Clone, Serialize)]
pub struct NftCleanupEntry {
    pub target: NftTarget,
    pub reason_code: CleanupReasonCode,
    pub reason: String,
    #[serde(skip)]
    pub prepared: Option<PreparedNftBurn>,
}
