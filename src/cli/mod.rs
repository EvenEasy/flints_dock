//! Transport adapter: arguments, dependency wiring, prompts and rendering.
pub mod args;
pub mod cleanup;
pub mod output;
pub mod scan;
pub mod swap;
use clap::{Parser, ValueEnum};
use std::process::ExitCode;
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum OutputFormat {
    Table,
    Json,
}
#[derive(clap::Subcommand)]
pub enum Command {
    /// Read wallet SOL, tokens and NFT inventory
    Scan(scan::ScanArgs),
    #[command(flatten)]
    Swap(swap::SwapCommand),
    /// Preview or explicitly execute swaps, burns and account closure
    Cleanup(cleanup::CleanupArgs),
}
#[derive(Parser)]
#[command(
    version,
    subcommand_negates_reqs = true,
    args_conflicts_with_subcommands = true,
    about = "Inspect a Solana wallet, swap tokens or execute a cleanup plan"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
    // Preserve the original `dock_flints -p WALLET` scan shorthand.
    #[command(flatten)]
    pub scan: scan::ScanArgs,
}
impl Cli {
    pub fn scan_options(&self) -> crate::core::ScanOptions {
        self.scan.scan_options()
    }
}
pub async fn run(cli: Cli) -> anyhow::Result<ExitCode> {
    match cli.command {
        Some(Command::Scan(args)) => scan::run(args).await,
        Some(Command::Swap(command)) => swap::run(command).await,
        Some(Command::Cleanup(args)) => cleanup::run(args).await,
        None => scan::run(cli.scan).await,
    }
}
