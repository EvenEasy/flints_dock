//! Local identity resolution. Seeds are raw Ed25519 seeds, not BIP39 phrases.
use base64::{Engine, engine::general_purpose::STANDARD};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use std::path::Path;

/// Keep signer material outside snapshots/plans and never derive Debug/Serialize.
pub struct WalletIdentity {
    pub address: Pubkey,
    signer: Option<Keypair>,
}
impl WalletIdentity {
    pub fn read_only(address: Pubkey) -> Self {
        Self {
            address,
            signer: None,
        }
    }
    fn signing(signer: Keypair) -> Self {
        Self {
            address: signer.pubkey(),
            signer: Some(signer),
        }
    }
    pub fn from_keypair(path: &Path) -> anyhow::Result<Self> {
        solana_keypair::read_keypair_file(path)
            .map(Self::signing)
            .map_err(|_| anyhow::anyhow!("cannot read a valid local Solana JSON keypair"))
    }
    pub fn from_seed(encoded: &str) -> anyhow::Result<Self> {
        // Fixed-size decoding prevents accepting truncated seeds or 64-byte keypairs.
        let mut seed = [0u8; 32];
        let length = STANDARD.decode_slice(encoded, &mut seed).map_err(|_| {
            anyhow::anyhow!("--seed must be base64 encoding of exactly 32 seed bytes")
        })?;
        anyhow::ensure!(length == 32, "--seed must encode exactly 32 seed bytes");
        Ok(Self::signing(Keypair::new_from_array(seed)))
    }
    pub fn signer(&self) -> anyhow::Result<&Keypair> {
        self.signer.as_ref().ok_or_else(|| {
            anyhow::anyhow!("execution requires --keypair or --seed; --pubkey is read-only")
        })
    }
}
