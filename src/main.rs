use clap::Parser;
#[tokio::main]
async fn main() -> anyhow::Result<std::process::ExitCode> {
    dock_flints::cli::run(dock_flints::cli::Cli::parse()).await
}
