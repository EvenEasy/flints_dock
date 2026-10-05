//! Scan command: resolve dependencies, invoke the use case, render the snapshot.
use super::{
    OutputFormat,
    args::{RpcArgs, WalletArgs},
};
use crate::{
    app::scan_wallet::scan_wallet,
    cli::output::{OutputOptions, console, json::portfolio_json},
    core::{ScanOptions, ScanSelection},
    infra::jupiter::Jupiter,
};
use clap::Args;
use std::{
    io::{self, Write},
    process::ExitCode,
};
#[derive(Args)]
pub struct ScanArgs {
    #[command(flatten)]
    pub wallet: WalletArgs,
    #[command(flatten)]
    pub rpc: RpcArgs,

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
}

impl ScanArgs {
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

/// Resolve read-only dependencies, scan selected categories and render the snapshot.
pub async fn run(args: ScanArgs) -> anyhow::Result<ExitCode> {
    let wallet = args.wallet.resolve()?;
    let options = args.scan_options();
    let provider = if options.selection.needs_prices() && !options.no_prices {
        std::env::var("JUPITER_API_KEY")
            .ok()
            .filter(|key| !key.trim().is_empty())
            .map(Jupiter::new)
            .transpose()
    } else {
        Ok(None)
    };
    let mut portfolio = scan_wallet(
        &args.rpc.client(),
        &wallet.address,
        &options,
        provider.as_ref().ok().and_then(Option::as_ref),
    )
    .await;
    if let Err(error) = provider {
        portfolio.scanners.insert(
            "prices".into(),
            crate::core::ScanStatus::Failed(error.to_string()),
        );
    }
    if args.verbose {
        for (name, status) in &portfolio.scanners {
            eprintln!("{name}: {status:?}");
        }
    }
    let mut out = io::BufWriter::new(io::stdout().lock());
    match args.format {
        OutputFormat::Json => {
            serde_json::to_writer_pretty(
                &mut out,
                &portfolio_json(&portfolio, &args.output_options()),
            )?;
            writeln!(out)?;
        }
        OutputFormat::Table => {
            console::write_portfolio(&mut out, &portfolio, &args.output_options())?
        }
    }
    out.flush()?;
    Ok(if portfolio.has_usable_results() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(2)
    })
}
