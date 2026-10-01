//! Shared argument groups. Parsing/configuration belongs here; business rules do not.
use crate::{core::swap::SwapLimits, infra::wallet::WalletIdentity};
use clap::Args;
use solana_pubkey::Pubkey;
use std::{path::PathBuf, time::Duration};

#[derive(Args)]
#[group(id = "wallet", required = true, multiple = false)]
#[command(group(clap::ArgGroup::new("signer").args(["keypair", "seed"]).multiple(false)))]
pub struct WalletArgs {
    /// Read-only wallet address
    #[arg(short, long, help_heading = "Wallet (choose one)")]
    pub pubkey: Option<Pubkey>,
    /// Local Solana JSON keypair; also derives the address for read-only commands
    #[arg(long, help_heading = "Wallet (choose one)")]
    pub keypair: Option<PathBuf>,
    /// Base64 of exactly 32 Ed25519 seed bytes; never transmitted to providers
    #[arg(long, help_heading = "Wallet (choose one)")]
    pub seed: Option<String>,
}
impl WalletArgs {
    pub fn resolve(&self) -> anyhow::Result<WalletIdentity> {
        // Validate here too so programmatic callers get the same identity contract.
        anyhow::ensure!(
            usize::from(self.pubkey.is_some())
                + usize::from(self.keypair.is_some())
                + usize::from(self.seed.is_some())
                == 1,
            "provide exactly one of --pubkey, --keypair or --seed"
        );
        if let Some(address) = self.pubkey {
            return Ok(WalletIdentity::read_only(address));
        }
        if let Some(path) = &self.keypair {
            return WalletIdentity::from_keypair(path);
        }
        WalletIdentity::from_seed(self.seed.as_deref().expect("validated identity"))
    }
}
#[derive(Args)]
pub struct RpcArgs {
    #[arg(
        short,
        long,
        default_value = "https://api.mainnet.solana.com",
        help_heading = "RPC"
    )]
    pub rpc_url: String,
    #[arg(long, default_value_t = 30, value_parser = clap::value_parser!(u64).range(1..=300), help_heading = "RPC")]
    pub timeout_seconds: u64,
}
impl RpcArgs {
    pub fn client(&self) -> solana_rpc_client::nonblocking::rpc_client::RpcClient {
        crate::infra::solana::client(self.rpc_url.clone(), self.timeout_seconds)
    }
}
#[derive(Args)]
#[group(id = "quote_policy")]
pub struct QuoteArgs {
    /// Slippage in basis points (50 = 0.5%)
    #[arg(long, default_value_t = 50, value_parser = clap::value_parser!(u16).range(1..=10000), help_heading = "Quote policy")]
    pub slippage_bps: u16,
    #[arg(long, default_value_t = 100, value_parser = clap::value_parser!(u16).range(1..=10000), help_heading = "Quote policy")]
    pub max_price_impact_bps: u16,
}
impl QuoteArgs {
    pub fn limits(&self) -> SwapLimits {
        SwapLimits {
            max_price_impact_bps: self.max_price_impact_bps,
            ..Default::default()
        }
    }
}
#[derive(Args)]
pub struct ExecutionArgs {
    #[arg(long, default_value_t = 90, value_parser = clap::value_parser!(u64).range(1..=600), help_heading = "Execution policy")]
    pub confirmation_timeout_seconds: u64,
    /// Priority fee cap; excludes base fee and rent
    #[arg(long, default_value_t = 1_000_000, help_heading = "Execution policy")]
    pub max_priority_fee_lamports: u64,
}
impl ExecutionArgs {
    pub fn limits(&self, quote: &QuoteArgs) -> SwapLimits {
        SwapLimits {
            confirmation_timeout: Duration::from_secs(self.confirmation_timeout_seconds),
            max_priority_fee_lamports: self.max_priority_fee_lamports,
            ..quote.limits()
        }
    }
}
pub fn swap_provider() -> anyhow::Result<crate::infra::jupiter::Jupiter> {
    crate::infra::jupiter::Jupiter::new(std::env::var("JUPITER_API_KEY").unwrap_or_default())
}
