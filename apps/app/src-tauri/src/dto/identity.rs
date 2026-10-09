use crate::error::AppError;
use dock_flints_core::infra::wallet::WalletIdentity;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// One identity source per request; credentials must never implement Debug or Serialize.
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum WalletSourceDto {
    PublicKey { address: String },
    Seed { base64: String },
    KeypairFile { path: String },
}

/// Local connection is separate from read-only analysis and transaction approval.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConnectWalletRequestDto {
    pub source: WalletSourceDto,
}

/// Public source label; this type contains no signer material.
#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum WalletSourceKindDto {
    PublicKey,
    Seed,
    KeypairFile,
}

/// Return public identity and capability only; signer material stays in backend memory.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletConnectionDto {
    pub session_id: String,
    pub wallet_address: String,
    pub source_kind: WalletSourceKindDto,
    pub can_sign: bool,
}

impl ConnectWalletRequestDto {
    pub fn source_kind(&self) -> WalletSourceKindDto {
        match self.source {
            WalletSourceDto::PublicKey { .. } => WalletSourceKindDto::PublicKey,
            WalletSourceDto::Seed { .. } => WalletSourceKindDto::Seed,
            WalletSourceDto::KeypairFile { .. } => WalletSourceKindDto::KeypairFile,
        }
    }

    /// Reuse the core loaders and replace library errors with credential-safe messages.
    pub fn into_identity(self) -> Result<WalletIdentity, AppError> {
        match self.source {
            WalletSourceDto::PublicKey { address } => address
                .parse()
                .map(WalletIdentity::read_only)
                .map_err(|_| AppError::invalid_wallet()),
            WalletSourceDto::Seed { base64 } => {
                if base64.len() != 44 {
                    return Err(AppError::invalid_identity(
                        "Seed must be standard base64 encoding of exactly 32 Ed25519 seed bytes",
                    ));
                }
                WalletIdentity::from_seed(&base64).map_err(|_| {
                    AppError::invalid_identity(
                        "Seed must be standard base64 encoding of exactly 32 Ed25519 seed bytes",
                    )
                })
            }
            WalletSourceDto::KeypairFile { path } => {
                let file = Path::new(&path);
                if path.len() > 4096 || !file.is_absolute() {
                    return Err(AppError::invalid_identity(
                        "Keypair file must use an absolute local path",
                    ));
                }
                let metadata = std::fs::metadata(file).map_err(|_| {
                    AppError::invalid_identity("Cannot read the local keypair file")
                })?;
                if !metadata.is_file() || metadata.len() > 4096 {
                    return Err(AppError::invalid_identity(
                        "Keypair must be a regular Solana JSON file of at most 4096 bytes",
                    ));
                }
                WalletIdentity::from_keypair(file).map_err(|_| {
                    AppError::invalid_identity("Cannot read a valid local Solana JSON keypair file")
                })
            }
        }
    }
}
