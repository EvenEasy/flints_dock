pub mod swap;
use crate::{
    models::{ScanOptions, ScanSelection},
    output::OutputOptions,
};
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
    subcommand_negates_reqs = true,
    args_conflicts_with_subcommands = true,
    about = "Scan Solana wallet assets (default --all) or swap one token to native SOL"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<swap::SwapCommand>,
    /// Wallet public key; no signing is needed
    #[arg(short, long, required = true)]
    pub pubkey: Option<Pubkey>,
    #[arg(short, long, default_value = "https://api.mainnet.solana.com")]
    pub rpc_url: String,
    /// Native SOL balance
    #[arg(long)]
    pub balance: bool,
    /// Fungible SPL / Token-2022 assets and unclassified tokens
    #[arg(long)]
    pub tokens: bool,
    /// Every token account, including NFTs and unknown assets (unaggregated)
    #[arg(long)]
    pub all_tokens: bool,
    /// Classic, programmable and MPL Core NFTs
    #[arg(long)]
    pub nfts: bool,
    /// Compressed NFTs (requires a historical owner index)
    #[arg(long)]
    pub cnfts: bool,
    /// All asset categories (raw --all-tokens view remains opt-in); also the default
    #[arg(long)]
    pub all: bool,
    /// Disable external price requests; otherwise uses JUPITER_API_KEY
    #[arg(long)]
    pub no_prices: bool,
    /// Display full token mints and NFT asset IDs in tables
    #[arg(long)]
    pub show_mint: bool,
    /// Display unit USD prices as well as values in tables
    #[arg(long)]
    pub show_price: bool,
    /// Include zero-balance token assets in output
    #[arg(long)]
    pub include_empty: bool,
    /// Show raw amounts, accounts, lamports and metadata
    #[arg(long)]
    pub details: bool,
    #[arg(long, value_enum, default_value = "table")]
    pub format: OutputFormat,
    /// Scanner diagnostics on stderr; does not expand asset tables
    #[arg(short, long)]
    pub verbose: bool,
    #[arg(long, default_value_t = 30, value_parser = clap::value_parser!(u64).range(1..=300))]
    pub timeout_seconds: u64,
}

impl Cli {
    pub fn scan_options(&self) -> ScanOptions {
        let selection = ScanSelection {
            balance: self.balance,
            tokens: self.tokens,
            all_tokens: self.all_tokens,
            nfts: self.nfts,
            cnfts: self.cnfts,
        };
        ScanOptions {
            selection: if self.all || selection.is_empty() {
                ScanSelection {
                    all_tokens: self.all_tokens,
                    ..ScanSelection::ALL
                }
            } else {
                selection
            },
            no_prices: self.no_prices,
            verbose: self.verbose,
        }
    }

    pub fn output_options(&self) -> OutputOptions {
        OutputOptions {
            show_mint: self.show_mint,
            show_price: self.show_price,
            include_empty: self.include_empty,
            details: self.details,
        }
    }
}
