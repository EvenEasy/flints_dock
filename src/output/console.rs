use crate::{models::*, portfolio::aggregate::exact_amount};
use std::io::{self, Write};

pub fn status(status: &ScanStatus) -> String {
    match status {
        ScanStatus::Complete => "Complete".into(),
        ScanStatus::Partial(reason) => format!("Partial: {reason}"),
        ScanStatus::Failed(reason) => format!("Failed: {reason}"),
        ScanStatus::Unsupported(reason) => format!("Unsupported: {reason}"),
        ScanStatus::Skipped(reason) => format!("Skipped: {reason}"),
    }
}
fn money(value: Option<f64>) -> String {
    value
        .map(|value| format!("${value:.2}"))
        .unwrap_or_else(|| "unavailable".into())
}
// Token names/URIs are untrusted on-chain text. Escape terminal control characters.
fn label(value: &str) -> String {
    value.chars().flat_map(char::escape_default).collect()
}

pub fn write_portfolio(
    mut out: impl Write,
    portfolio: &Portfolio,
    verbose: bool,
) -> io::Result<()> {
    writeln!(
        out,
        "Wallet: {}\nCommitment: {}",
        portfolio.owner, portfolio.commitment
    )?;
    writeln!(out, "\nSOL")?;
    if let Some(native) = &portfolio.native_sol {
        writeln!(
            out,
            "  Balance: {} SOL ({} lamports)\n  USD: {}",
            exact_amount(native.lamports.into(), 9),
            native.lamports,
            money(native.value_usd)
        )?;
    } else {
        writeln!(out, "  Balance unavailable; see scanner status")?;
    }
    writeln!(
        out,
        "\nTOKENS (includes NFT token balances; not counted again in valuation)"
    )?;
    for token in &portfolio.tokens {
        writeln!(
            out,
            "  {} [{:?}]\n    mint: {}\n    balance: {} (raw={}, decimals={:?})\n    price: {}  value: {}  accounts: {}",
            label(
                token
                    .metadata
                    .symbol
                    .as_deref()
                    .or(token.metadata.name.as_deref())
                    .unwrap_or("UNKNOWN TOKEN")
            ),
            token.kind,
            token.mint,
            token.balance.as_deref().unwrap_or("unavailable"),
            token.total_raw_amount,
            token.decimals,
            money(token.price.as_ref().map(|price| price.usd)),
            money(token.value_usd),
            token.accounts.len()
        )?;
    }
    writeln!(
        out,
        "\nNFT: {} discovered ({} programmable)",
        portfolio.classic_nfts.len(),
        portfolio
            .classic_nfts
            .iter()
            .filter(|nft| nft.programmable)
            .count()
    )?;
    for nft in &portfolio.classic_nfts {
        writeln!(
            out,
            "  {} {} (edition={})",
            nft.mint,
            label(nft.metadata.name.as_deref().unwrap_or("unnamed")),
            nft.edition
        )?;
    }
    writeln!(
        out,
        "\nMPL CORE: {} discovered AssetV1",
        portfolio.core_assets.len()
    )?;
    for core in &portfolio.core_assets {
        writeln!(
            out,
            "  {} {} ({} lamports)",
            core.address,
            label(&core.name),
            core.lamports
        )?;
    }
    writeln!(
        out,
        "\nSTAKE: {} authority-associated accounts",
        portfolio.stake_accounts.len()
    )?;
    for stake in &portfolio.stake_accounts {
        writeln!(
            out,
            "  {}: {} SOL; delegated={} lamports; withdraw authority={}; vote={}",
            stake.address,
            exact_amount(stake.lamports.into(), 9),
            stake
                .delegated_lamports
                .map(|amount| amount.to_string())
                .unwrap_or_else(|| "none".into()),
            stake.wallet_can_withdraw,
            stake.validator_vote_account.as_deref().unwrap_or("none")
        )?;
    }
    writeln!(
        out,
        "\nNONCE ACCOUNTS: {} discovered",
        portfolio.associated_accounts.len()
    )?;
    for account in &portfolio.associated_accounts {
        writeln!(out, "  {}: {} lamports", account.address, account.lamports)?;
    }
    let summary = &portfolio.account_summary;
    writeln!(
        out,
        "\nTOKEN ACCOUNTS\n  Total decoded: {}\n  Empty public balances: {}\n  Lamports stored: {} ({} SOL)\n  Potentially reclaimable from empty accounts: {} lamports ({} SOL; conditional, before fees)\n  Closure needs extension review: {}",
        summary.token_accounts,
        summary.empty_token_accounts,
        summary.token_account_lamports,
        exact_amount(summary.token_account_lamports, 9),
        summary.potentially_reclaimable_lamports,
        exact_amount(summary.potentially_reclaimable_lamports, 9),
        summary.closure_review_accounts
    )?;
    if verbose {
        for account in &portfolio.token_accounts {
            writeln!(
                out,
                "  {} mint={} raw={} lamports={} close={:?}",
                account.address,
                account.mint,
                account.raw_amount,
                account.lamports,
                account.closure
            )?;
        }
    }
    writeln!(
        out,
        "\nUNKNOWN / UNCLASSIFIED: {}",
        portfolio.unknown_assets.len()
    )?;
    for asset in &portfolio.unknown_assets {
        writeln!(out, "  {}: {}", asset.address, label(&asset.reason))?;
    }
    writeln!(out, "\nSCAN STATUS")?;
    for (scanner, state) in &portfolio.scanners {
        writeln!(out, "  {scanner}: {}", label(&status(state)))?;
    }
    writeln!(
        out,
        "\nPORTFOLIO\n  Known USD value: {}\n  {}",
        money(portfolio.known_value_usd),
        portfolio.valuation_scope
    )?;
    for limitation in &portfolio.limitations {
        writeln!(out, "  Note: {limitation}")?;
    }
    Ok(())
}
