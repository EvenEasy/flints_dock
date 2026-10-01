use crate::{
    cleanup::{self, *},
    cli::OutputFormat,
    jupiter::Jupiter,
    models::*,
    output::cleanup::{write_plan, write_report},
    portfolio::service::scan_wallet,
};
use clap::Args;
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use std::{
    io::{self, IsTerminal, Write},
    path::PathBuf,
    process::ExitCode,
    time::Duration,
};

#[derive(Args)]
pub struct CleanupArgs {
    #[arg(short, long)]
    pub pubkey: Pubkey,
    /// Explicitly execute the displayed swaps, burns and closes; default is preview
    #[arg(long, requires = "keypair", conflicts_with = "dry_run")]
    pub execute: bool,
    #[arg(long)]
    pub dry_run: bool,
    #[arg(long, requires = "execute")]
    pub keypair: Option<PathBuf>,
    /// Approve the displayed cleanup, including irreversible burns
    #[arg(long, requires = "execute")]
    pub yes: bool,
    /// Limit cleanup to these token account addresses; repeat to select several
    #[arg(long)]
    pub account: Vec<Pubkey>,
    #[arg(short, long, default_value = "https://api.mainnet.solana.com")]
    pub rpc_url: String,
    #[arg(long, default_value_t = 30, value_parser = clap::value_parser!(u64).range(1..=300))]
    pub timeout_seconds: u64,
    #[arg(long, default_value_t = 90, value_parser = clap::value_parser!(u64).range(1..=600))]
    pub confirmation_timeout_seconds: u64,
    #[arg(long, default_value_t = 50, value_parser = clap::value_parser!(u16).range(1..=10000))]
    pub slippage_bps: u16,
    #[arg(long, default_value_t = 100, value_parser = clap::value_parser!(u16).range(1..=10000))]
    pub max_price_impact_bps: u16,
    #[arg(long, default_value_t = 1_000_000)]
    pub max_priority_fee_lamports: u64,
    /// Delay between quote attempts; default accommodates keyless Jupiter access
    #[arg(long, default_value_t = 2100, value_parser = clap::value_parser!(u64).range(0..=60000))]
    pub quote_interval_ms: u64,
    #[arg(long, value_enum, default_value = "table")]
    pub format: OutputFormat,
    #[arg(short, long)]
    pub verbose: bool,
}
async fn run_inner(args: CleanupArgs) -> anyhow::Result<ExitCode> {
    let signer = if args.execute {
        let signer =
            solana_keypair::read_keypair_file(args.keypair.as_ref().expect("clap keypair"))
                .map_err(|_| anyhow::anyhow!("cannot read a valid local Solana JSON keypair"))?;
        anyhow::ensure!(
            signer.pubkey() == args.pubkey,
            "keypair does not match --pubkey"
        );
        Some(signer)
    } else {
        None
    };
    let rpc = crate::rpc::client(args.rpc_url, args.timeout_seconds);
    let provider = Jupiter::new(std::env::var("JUPITER_API_KEY").unwrap_or_default())?;
    let options = CleanupOptions {
        slippage_bps: args.slippage_bps,
        quote_interval: Duration::from_millis(args.quote_interval_ms),
        swap_limits: crate::swap::SwapLimits {
            max_price_impact_bps: args.max_price_impact_bps,
            max_priority_fee_lamports: args.max_priority_fee_lamports,
            confirmation_timeout: Duration::from_secs(args.confirmation_timeout_seconds),
            ..Default::default()
        },
        ..Default::default()
    };
    eprintln!("Scanning token accounts and building cleanup plan (sequential quotes)...");
    let portfolio = scan_wallet(
        &rpc,
        &args.pubkey,
        &ScanOptions {
            selection: ScanSelection {
                all_tokens: true,
                ..Default::default()
            },
            no_prices: true,
            verbose: args.verbose,
        },
        None,
    )
    .await;
    let selected = |address: &str| {
        args.account.is_empty() || args.account.iter().any(|key| key.to_string() == address)
    };
    let assets = plan::assets_from_portfolio(&portfolio)
        .into_iter()
        .filter(|asset| selected(&asset.account.address))
        .collect();
    let unknown: Vec<_> = portfolio
        .unknown_assets
        .iter()
        .filter(|asset| {
            asset.data.is_some() && asset.lamports.is_some() && selected(&asset.address)
        })
        .cloned()
        .collect();
    for requested in &args.account {
        anyhow::ensure!(
            portfolio
                .token_accounts
                .iter()
                .any(|account| account.address == requested.to_string())
                || unknown
                    .iter()
                    .any(|account| account.address == requested.to_string()),
            "requested account {requested} was not discovered for this wallet"
        );
    }
    let status = portfolio
        .scanners
        .get("all_tokens")
        .cloned()
        .unwrap_or(ScanStatus::Failed("discovery did not run".into()));
    let usable = !matches!(status, ScanStatus::Failed(_));
    let plan = plan::build_plan(&args.pubkey, assets, status, unknown, &provider, &options).await;
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
    let report = cleanup::service::execute_plan(
        &plan,
        &provider,
        &rpc,
        signer.as_ref().expect("execution signer"),
        &options,
    )
    .await?;
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
