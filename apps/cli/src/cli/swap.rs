use super::{
    OutputFormat,
    args::{ExecutionArgs, QuoteArgs as QuotePolicy, RpcArgs, WalletArgs},
};
use crate::app::swap::{self, SwapError, SwapRequest};
use clap::{Args, Subcommand};
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use std::{
    io::{self, IsTerminal, Write},
    process::ExitCode,
};
#[derive(Subcommand)]
pub enum SwapCommand {
    /// Preview a real token/native SOL route
    Quote(QuoteArgs),

    /// Preview, approve, rebuild and execute one swap
    Swap(ExecuteArgs),
}
#[derive(Args)]
pub struct SwapArgs {
    /// Exact mint address, never a symbol
    #[arg(long)]
    pub mint: Pubkey,

    /// Integer token base units, before decimals
    #[arg(long, value_parser = clap::value_parser!(u64).range(1..))]
    pub raw_amount: u64,
    #[command(flatten)]
    pub quote: QuotePolicy,
    #[arg(long, value_enum, default_value = "table")]
    pub format: OutputFormat,
}
#[derive(Args)]
pub struct QuoteArgs {
    #[command(flatten)]
    pub swap: SwapArgs,
    #[command(flatten)]
    pub wallet: WalletArgs,
}
#[derive(Args)]
#[command(group(clap::ArgGroup::new("execution_identity").args(["keypair", "seed"]).required(true)))]
pub struct ExecuteArgs {
    #[command(flatten)]
    pub swap: SwapArgs,
    #[command(flatten)]
    pub wallet: WalletArgs,
    #[command(flatten)]
    pub rpc: RpcArgs,
    #[command(flatten)]
    pub execution: ExecutionArgs,

    /// Approve the displayed minimum without an interactive prompt
    #[arg(long)]
    pub yes: bool,
}
fn request(args: &SwapArgs, wallet: Pubkey) -> SwapRequest {
    SwapRequest {
        mint: args.mint,
        raw_amount: args.raw_amount,
        wallet,
        slippage_bps: args.quote.slippage_bps,
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
async fn run_inner(command: SwapCommand) -> swap::Result<serde_json::Value> {
    let provider = super::args::swap_provider().map_err(|e| SwapError::Api(e.to_string()))?;
    match command {
        SwapCommand::Quote(args) => {
            let wallet = args
                .wallet
                .resolve()
                .map_err(|e| SwapError::InvalidRequest(e.to_string()))?;
            let limits = args.swap.quote.limits();
            let preview =
                swap::preview(&provider, &request(&args.swap, wallet.address), &limits).await?;
            if matches!(args.swap.format, OutputFormat::Table) {
                write_quote(io::stdout().lock(), &preview.quote)
                    .map_err(|e| SwapError::InvalidRequest(e.to_string()))?;
            }
            Ok(serde_json::json!({"status":"quoted", "quote":preview.quote}))
        }
        SwapCommand::Swap(args) => {
            let wallet = args
                .wallet
                .resolve()
                .map_err(|e| SwapError::InvalidRequest(e.to_string()))?;
            let signer = wallet
                .signer()
                .map_err(|e| SwapError::InvalidRequest(e.to_string()))?;
            let request = request(&args.swap, wallet.address);
            let limits = args.execution.limits(&args.swap.quote);
            let rpc = args.rpc.client();
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
            let receipt = swap::execute(&provider, &rpc, &preview, signer, &limits).await?;
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

/// Render swap results or semantic errors without mixing diagnostics into JSON stdout.
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
