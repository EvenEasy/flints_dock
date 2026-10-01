use anyhow::Result;
use clap::Parser;
use dock_flints::{
    cli::{Cli, OutputFormat},
    output::{console, json::portfolio_json},
    portfolio::service::scan_wallet,
};
use std::{
    io::{self, Write},
    process::ExitCode,
};

#[tokio::main]
async fn main() -> Result<ExitCode> {
    let args = Cli::parse();
    if let Some(command) = args.command {
        return match command {
            dock_flints::cli::Command::Swap(command) => dock_flints::cli::swap::run(command).await,
            dock_flints::cli::Command::Cleanup(args) => dock_flints::cli::cleanup::run(args).await,
        };
    }
    let options = args.scan_options();
    let output_options = args.output_options();
    let rpc = dock_flints::rpc::client(args.rpc_url, args.timeout_seconds);
    let portfolio = scan_wallet(
        &rpc,
        &args.pubkey.expect("clap requires a wallet for scans"),
        &options,
        std::env::var("JUPITER_API_KEY").ok(),
    )
    .await;
    let mut out = io::BufWriter::new(io::stdout().lock());
    match args.format {
        OutputFormat::Json => {
            serde_json::to_writer_pretty(&mut out, &portfolio_json(&portfolio, &output_options))?;
            writeln!(out)?;
        }
        OutputFormat::Table => console::write_portfolio(&mut out, &portfolio, &output_options)?,
    }
    out.flush()?;
    // Render machine-readable failure first, then return an unsuccessful exit code.
    Ok(if portfolio.has_usable_results() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(2)
    })
}
