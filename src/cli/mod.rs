use clap::{Parser, ValueEnum};
use solana_pubkey::Pubkey;

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum OutputFormat {
    Table,
    Json,
}

#[derive(Parser)]
#[command(
    version,
    about = "Discover a Solana wallet portfolio using standard RPC"
)]
pub struct Cli {
    /// Wallet public key; no private key or signing is needed
    #[arg(short, long)]
    pub pubkey: Pubkey,
    /// Solana JSON-RPC endpoint
    #[arg(short, long, default_value = "https://api.mainnet.solana.com")]
    pub rpc_url: String,
    /// Disable optional Jupiter pricing (otherwise uses JUPITER_API_KEY)
    #[arg(long)]
    pub no_prices: bool,
    #[arg(long, value_enum, default_value = "table")]
    pub format: OutputFormat,
    /// Progress on stderr; JSON stdout remains clean
    #[arg(short, long)]
    pub verbose: bool,
    /// Per-request timeout in seconds
    #[arg(long, default_value_t = 30, value_parser = clap::value_parser!(u64).range(1..=300))]
    pub timeout_seconds: u64,
}
