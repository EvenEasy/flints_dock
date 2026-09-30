use anyhow::Result;
use clap::Parser;
use dock_flints::{
    cli::{Cli, OutputFormat},
    output::{console, json::portfolio_json},
    portfolio::service::scan_wallet,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_commitment_config::CommitmentConfig;
use std::{
    io::{self, Write},
    process::ExitCode,
    time::Duration,
};

#[tokio::main]
async fn main() -> Result<ExitCode> {
    let args = Cli::parse();
    let options = args.scan_options();
    let output_options = args.output_options();
    let rpc = RpcClient::new_with_timeout_and_commitment(
        args.rpc_url,
        Duration::from_secs(args.timeout_seconds),
        CommitmentConfig::confirmed(),
    );
    let portfolio = scan_wallet(
        &rpc,
        &args.pubkey,
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
