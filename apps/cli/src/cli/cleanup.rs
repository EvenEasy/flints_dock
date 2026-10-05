use crate::{
    app::cleanup::{self, *},
    cli::{
        OutputFormat,
        args::{ExecutionArgs, QuoteArgs, RpcArgs, WalletArgs},
        output::cleanup::{write_plan, write_report},
    },
};
use clap::Args;
use solana_pubkey::Pubkey;
use std::{
    io::{self, IsTerminal, Write},
    process::ExitCode,
    time::Duration,
};
#[derive(Args)]
pub struct CleanupArgs {
    #[command(flatten)]
    pub wallet: WalletArgs,

    /// Execute the displayed plan, including irreversible burns
    #[arg(long, requires = "signer", conflicts_with = "dry_run")]
    pub execute: bool,

    /// Explicit preview; never signs or submits transactions
    #[arg(long)]
    pub dry_run: bool,

    /// Approve the displayed cleanup, including irreversible burns
    #[arg(long, requires = "execute")]
    pub yes: bool,

    /// Restrict cleanup to these account addresses; repeat to select several
    #[arg(long, help_heading = "Selection")]
    pub account: Vec<Pubkey>,

    /// Protect every account of this mint from swap, burn and close; repeatable
    #[arg(long, help_heading = "Selection")]
    pub ignore_mint: Vec<Pubkey>,
    #[command(flatten)]
    pub rpc: RpcArgs,
    #[command(flatten)]
    pub quote: QuoteArgs,
    #[command(flatten)]
    pub execution: ExecutionArgs,

    /// Delay between quote attempts, including keyless provider throttling
    #[arg(long, default_value_t = 2100, value_parser = clap::value_parser!(u64).range(0..=60000), help_heading = "Quote policy")]
    pub quote_interval_ms: u64,
    #[arg(long, value_enum, default_value = "table")]
    pub format: OutputFormat,

    /// Print discovery diagnostics on stderr
    #[arg(short, long)]
    pub verbose: bool,
}
async fn run_inner(args: CleanupArgs) -> anyhow::Result<ExitCode> {
    let wallet = args.wallet.resolve()?;
    if args.execute {
        wallet.signer()?;
    }
    let rpc = args.rpc.client();
    let provider = super::args::swap_provider()?;
    let options = CleanupOptions {
        slippage_bps: args.quote.slippage_bps,
        quote_interval: Duration::from_millis(args.quote_interval_ms),
        swap_limits: args.execution.limits(&args.quote),
        selection: CleanupSelection {
            accounts: args.account.iter().map(ToString::to_string).collect(),
            ignored_mints: args.ignore_mint.iter().map(ToString::to_string).collect(),
        },
        ..Default::default()
    };
    eprintln!("Scanning token accounts and building cleanup plan (sequential quotes)...");
    let plan = cleanup::plan_wallet(&rpc, &wallet.address, &provider, &options).await?;
    let usable = !matches!(plan.discovery_status, crate::core::ScanStatus::Failed(_));
    if args.verbose {
        eprintln!("Discovery: {:?}", plan.discovery_status);
    }
    if args.execute {
        write_plan(io::stderr().lock(), &plan)?;
    } else {
        match args.format {
            OutputFormat::Table => write_plan(io::stdout().lock(), &plan)?,
            OutputFormat::Json => println!(
                "{}",
                serde_json::to_string_pretty(
                    &serde_json::json!({"mode":"dry_run","transactions_submitted":0,"plan":plan})
                )?
            ),
        }
        return Ok(if usable {
            ExitCode::SUCCESS
        } else {
            ExitCode::from(2)
        });
    }
    anyhow::ensure!(usable, "token discovery failed; nothing executed");

    // The displayed plan is read-only until the user explicitly approves its destructive
    // operations.
    if !args.yes && plan.summary.accounts_to_close > 0 {
        anyhow::ensure!(
            io::stdin().is_terminal(),
            "use a terminal to confirm cleanup, or explicitly pass --execute --yes"
        );
        eprint!(
            "Execute listed swaps, permanently burn listed balances, and close accounts? Type cleanup: "
        );
        io::stderr().flush()?;
        let mut answer = String::new();
        io::stdin().read_line(&mut answer)?;
        if answer.trim() != "cleanup" {
            match args.format {
                OutputFormat::Json => {
                    println!("{{\"status\":\"cancelled\",\"transactions_submitted\":0}}")
                }
                OutputFormat::Table => println!("Cleanup cancelled; no transactions submitted."),
            }
            return Ok(ExitCode::SUCCESS);
        }
    }
    let report =
        cleanup::execute::execute_plan(&plan, &provider, &rpc, wallet.signer()?, &options).await?;
    let failed = report.failed > 0;
    match args.format {
        OutputFormat::Table => write_report(io::stdout().lock(), &report)?,
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&report)?),
    }
    Ok(if failed {
        ExitCode::from(2)
    } else {
        ExitCode::SUCCESS
    })
}

/// Render cleanup failures in the requested format and return the matching exit code.
pub async fn run(args: CleanupArgs) -> anyhow::Result<ExitCode> {
    let format = args.format;
    match run_inner(args).await {
        Ok(code) => Ok(code),
        Err(error) => {
            if matches!(format, OutputFormat::Json) {
                println!(
                    "{}",
                    serde_json::json!({"status":"error","error":error.to_string()})
                );
            } else {
                eprintln!("Cleanup failed: {error}");
            }
            Ok(ExitCode::from(2))
        }
    }
}
