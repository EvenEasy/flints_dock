use anyhow::Result;
use clap::Parser;
use flints_station::{
    cli::{Cli, OutputFormat},
    output::console,
    portfolio::service::scan_wallet,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_commitment_config::CommitmentConfig;
use std::{
    io::{self, Write},
    time::Duration,
};

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();
    let rpc = RpcClient::new_with_timeout_and_commitment(
        args.rpc_url,
        Duration::from_secs(args.timeout_seconds),
        CommitmentConfig::confirmed(),
    );
    let portfolio = scan_wallet(
        &rpc,
        &args.pubkey,
        args.no_prices,
        std::env::var("JUPITER_API_KEY").ok(),
        args.verbose,
    )
    .await?;
    let mut out = io::BufWriter::new(io::stdout().lock());
    match args.format {
        OutputFormat::Json => {
            serde_json::to_writer_pretty(&mut out, &portfolio)?;
            writeln!(out)?;
        }
        OutputFormat::Table => console::write_portfolio(&mut out, &portfolio, args.verbose)?,
    }
    out.flush()?;
    Ok(())
}
