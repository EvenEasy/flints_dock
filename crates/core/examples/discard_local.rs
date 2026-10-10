//! Disposable validator integration only. Never accepts an external RPC endpoint.
use dock_flints_core::{
    app::{categories::ScopedSwap, cleanup::plan_wallet},
    core::{cleanup::*, progress::*},
    infra::{solana, wallet::WalletIdentity},
};
use std::collections::BTreeSet;
struct Journal(std::path::PathBuf);
impl CleanupObserver for Journal {
    fn before_send(
        &self,
        submission: PendingSubmission,
    ) -> Result<(), dock_flints_core::core::error::SwapError> {
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(&self.0)
            .map_err(|e| dock_flints_core::core::error::SwapError::InvalidRequest(e.to_string()))?;
        writeln!(file, "{}", serde_json::to_string(&submission).unwrap())
            .map_err(|e| dock_flints_core::core::error::SwapError::InvalidRequest(e.to_string()))?;
        file.sync_all()
            .map_err(|e| dock_flints_core::core::error::SwapError::InvalidRequest(e.to_string()))
    }
}
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<_> = std::env::args().collect();
    anyhow::ensure!(
        args.len() == 4,
        "Use RPC KEYPAIR ACCOUNT for an owned disposable local fixture"
    );
    let url = reqwest::Url::parse(&args[1])?;
    anyhow::ensure!(
        url.scheme() == "http" && matches!(url.host_str(), Some("127.0.0.1") | Some("localhost")),
        "This integration helper only permits a local validator"
    );
    let identity = WalletIdentity::from_keypair(std::path::Path::new(&args[2]))?;
    let rpc = solana::client(args[1].clone(), 30);
    let provider = ScopedSwap::<dock_flints_core::infra::jupiter::Jupiter> {
        provider: None,
        mainnet: false,
    };
    let options = CleanupOptions {
        policy: CleanupPolicy::ExplicitDiscard,
        selection: CleanupSelection {
            accounts: BTreeSet::from([args[3].clone()]),
            ..Default::default()
        },
        quote_interval: std::time::Duration::ZERO,
        ..Default::default()
    };
    let plan = plan_wallet(&rpc, &identity.address, &provider, &options).await?;
    anyhow::ensure!(
        plan.summary.burnable == 1 && plan.summary.accounts_to_close == 1,
        "Fixture must contain one nonempty eligible source"
    );
    let observer = Journal(std::path::Path::new(&args[2]).with_file_name("submissions.jsonl"));
    let report = dock_flints_core::app::cleanup::execute::execute_plan_observed(
        &plan,
        &provider,
        &rpc,
        identity.signer()?,
        &options,
        &observer,
    )
    .await?;
    println!(
        "{}",
        serde_json::to_string_pretty(
            &serde_json::json!({"network": rpc.get_genesis_hash().await?.to_string(), "wallet":identity.address.to_string(), "plan":plan, "report":report})
        )?
    );
    anyhow::ensure!(
        report.closed == 1 && report.failed == 0,
        "Disposable burn/close must confirm and verify account absence"
    );
    Ok(())
}
