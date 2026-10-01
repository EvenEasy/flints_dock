use super::OutputFormat;
use crate::{
    jupiter::Jupiter,
    swap::{self, SwapError, SwapLimits, SwapRequest},
};
use clap::{Args, Subcommand};
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use std::{
    io::{self, IsTerminal, Write},
    path::PathBuf,
    process::ExitCode,
    time::Duration,
};

#[derive(Subcommand)]
pub enum SwapCommand {
    /// Preview a real TOKEN/native SOL route; does not require a keypair
    Quote(QuoteArgs),
    /// Preview, confirm, fetch a fresh route, then sign and submit one swap
    Swap(ExecuteArgs),
}
#[derive(Args)]
pub struct SwapArgs {
    /// Exact input token mint address (never a symbol)
    #[arg(long)]
    pub mint: Pubkey,
    /// Integer token base units, before decimals
    #[arg(long, value_parser = clap::value_parser!(u64).range(1..))]
    pub raw_amount: u64,
    /// Maximum slippage in basis points (50 = 0.5%)
    #[arg(long, default_value_t = 50, value_parser = clap::value_parser!(u16).range(1..=10000))]
    pub slippage_bps: u16,
    /// Maximum absolute route price impact (100 = 1%)
    #[arg(long, default_value_t = 100, value_parser = clap::value_parser!(u16).range(1..=10000))]
    pub max_price_impact_bps: u16,
    #[arg(long, value_enum, default_value = "table")]
    pub format: OutputFormat,
}
#[derive(Args)]
pub struct QuoteArgs {
    #[command(flatten)]
    pub swap: SwapArgs,
    /// Taker public key required by Jupiter /build; no private key is read
    #[arg(short, long)]
    pub pubkey: Pubkey,
}
#[derive(Args)]
pub struct ExecuteArgs {
    #[command(flatten)]
    pub swap: SwapArgs,
    /// Local Solana CLI JSON keypair file; never sent to Jupiter or RPC
    #[arg(long)]
    pub keypair: PathBuf,
    #[arg(short, long, default_value = "https://api.mainnet.solana.com")]
    pub rpc_url: String,
    #[arg(long, default_value_t = 30, value_parser = clap::value_parser!(u64).range(1..=300))]
    pub timeout_seconds: u64,
    #[arg(long, default_value_t = 90, value_parser = clap::value_parser!(u64).range(1..=600))]
    pub confirmation_timeout_seconds: u64,
    /// Cap the transaction's priority fee, separate from base fees/account rent
    #[arg(long, default_value_t = 1_000_000)]
    pub max_priority_fee_lamports: u64,
    /// Approve the displayed preview minimum without an interactive prompt
    #[arg(long)]
    pub yes: bool,
}

fn request(args: &SwapArgs, wallet: Pubkey) -> SwapRequest {
    SwapRequest {
        mint: args.mint,
        raw_amount: args.raw_amount,
        wallet,
        slippage_bps: args.slippage_bps,
    }
}
fn write_quote(mut out: impl Write, quote: &swap::SwapQuote) -> io::Result<()> {
    writeln!(
        out,
        "Mint: {}\nRaw amount: {}\nOutput: native SOL\nExpected: {} SOL\nMinimum: {} SOL\nPrice impact: {:.4}%\nRoute: available\nOutput amounts are before network fees and account rent.",
        quote.input_mint,
        quote.raw_amount,
        quote.expected_out_sol,
        quote.min_out_sol,
        quote.price_impact_pct
    )
}
fn provider() -> swap::Result<Jupiter> {
    // Swap V2 supports limited keyless access; reuse the configured key when present.
    let key = std::env::var("JUPITER_API_KEY").unwrap_or_default();
    Jupiter::new(key).map_err(|e| SwapError::Api(e.to_string()))
}
async fn run_inner(command: SwapCommand) -> swap::Result<serde_json::Value> {
    let provider = provider()?;
    match command {
        SwapCommand::Quote(args) => {
            let limits = SwapLimits {
                max_price_impact_bps: args.swap.max_price_impact_bps,
                ..Default::default()
            };
            let preview =
                swap::preview(&provider, &request(&args.swap, args.pubkey), &limits).await?;
            if matches!(args.swap.format, OutputFormat::Table) {
                write_quote(io::stdout().lock(), &preview.quote)
                    .map_err(|e| SwapError::InvalidRequest(e.to_string()))?;
            }
            Ok(serde_json::json!({"status":"quoted", "quote":preview.quote}))
        }
        SwapCommand::Swap(args) => {
            let signer = solana_keypair::read_keypair_file(&args.keypair).map_err(|_| {
                SwapError::InvalidRequest(
                    "cannot read a valid Solana JSON keypair from --keypair".into(),
                )
            })?;
            let request = request(&args.swap, signer.pubkey());
            let limits = SwapLimits {
                max_price_impact_bps: args.swap.max_price_impact_bps,
                max_priority_fee_lamports: args.max_priority_fee_lamports,
                confirmation_timeout: Duration::from_secs(args.confirmation_timeout_seconds),
                ..Default::default()
            };
            let rpc = crate::rpc::client(args.rpc_url, args.timeout_seconds);
            let preview = swap::preview(&provider, &request, &limits).await?;
            // Preview/prompt are on stderr: stdout remains one JSON result when requested.
            write_quote(io::stderr().lock(), &preview.quote)
                .map_err(|e| SwapError::InvalidRequest(e.to_string()))?;
            eprintln!("Wallet: {}", signer.pubkey());
            if !args.yes {
                if !io::stdin().is_terminal() {
                    return Err(SwapError::InvalidRequest("interactive confirmation requires a terminal; use --yes to explicitly approve the preview minimum".into()));
                }
                eprint!(
                    "Swap this mint and amount for at least {} SOL before fees? [y/N] ",
                    preview.quote.min_out_sol
                );
                io::stderr()
                    .flush()
                    .map_err(|e| SwapError::InvalidRequest(e.to_string()))?;
                let mut answer = String::new();
                io::stdin()
                    .read_line(&mut answer)
                    .map_err(|e| SwapError::InvalidRequest(e.to_string()))?;
                if !matches!(answer.trim().to_ascii_lowercase().as_str(), "y" | "yes") {
                    return Ok(serde_json::json!({"status":"cancelled"}));
                }
            }
            let receipt = swap::execute(&provider, &rpc, &preview, &signer, &limits).await?;
            if matches!(args.swap.format, OutputFormat::Table) {
                println!(
                    "Confirmed native SOL swap: {}\nFresh quote: {} SOL; minimum {} SOL (before fees)",
                    receipt.signature,
                    receipt.fresh_quote.expected_out_sol,
                    receipt.fresh_quote.min_out_sol
                );
            }
            serde_json::to_value(receipt).map_err(|e| SwapError::InvalidResponse(e.to_string()))
        }
    }
}

pub async fn run(command: SwapCommand) -> anyhow::Result<ExitCode> {
    let format = match &command {
        SwapCommand::Quote(args) => args.swap.format,
        SwapCommand::Swap(args) => args.swap.format,
    };
    let result = run_inner(command).await;
    match result {
        Ok(value) => {
            if matches!(format, OutputFormat::Json) {
                println!("{}", serde_json::to_string_pretty(&value)?);
            } else if value["status"] == "cancelled" {
                println!("Swap cancelled");
            }
            Ok(ExitCode::SUCCESS)
        }
        Err(error) => {
            if matches!(format, OutputFormat::Json) {
                let signature = match &error {
                    SwapError::Uncertain { signature, .. }
                    | SwapError::TransactionFailed { signature, .. } => Some(signature.as_str()),
                    _ => None,
                };
                let route_exists = if matches!(error, SwapError::NoRoute(_)) {
                    Some(false)
                } else {
                    None
                };
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "status":"error", "error":error.to_string(), "route_exists":route_exists, "signature":signature
                    }))?
                );
            } else {
                eprintln!("{error}");
            }
            Ok(ExitCode::from(2))
        }
    }
}
