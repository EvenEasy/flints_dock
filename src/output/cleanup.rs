use crate::{cleanup::*, portfolio::aggregate::exact_amount};
use std::io::{self, Write};
fn safe(text: &str) -> String {
    text.chars()
        .flat_map(|c| {
            if c.is_control() {
                c.escape_default().collect::<Vec<_>>()
            } else {
                vec![c]
            }
        })
        .collect()
}
pub fn write_plan(mut out: impl Write, plan: &CleanupPlan) -> io::Result<()> {
    let s = &plan.summary;
    writeln!(
        out,
        "Wallet cleanup plan\nWallet: {}\n\nToken accounts: {}\n\nEmpty:       {}\nSwappable:   {}\nBurnable:    {}\nUnsupported: {}\n\nEstimated swap output: {} SOL\nAccounts to close: {}\nEstimated reclaimed lamports: {}\n",
        plan.wallet,
        s.token_accounts,
        s.empty,
        s.swappable,
        s.burnable,
        s.unsupported,
        exact_amount(s.estimated_swap_lamports, 9),
        s.accounts_to_close,
        s.estimated_reclaimed_lamports
    )?;
    if !plan.discovery_status.is_complete() {
        writeln!(
            out,
            "Inventory is incomplete; see JSON discovery_status for details."
        )?;
    }
    for entry in &plan.entries {
        let action = match entry.category {
            CleanupCategory::Empty => "CLOSE",
            CleanupCategory::Swappable => "SWAP -> CLOSE",
            CleanupCategory::Burnable => "BURN -> CLOSE",
            CleanupCategory::Unsupported => "SKIP",
        };
        writeln!(
            out,
            "{action}  account={}  mint={}  raw={}",
            entry.asset.account.address, entry.asset.account.mint, entry.asset.account.raw_amount
        )?;
        if let Some(quote) = &entry.quote {
            writeln!(
                out,
                "  expected={} SOL  minimum={} SOL  impact={:.4}%",
                quote.expected_out_sol, quote.min_out_sol, quote.price_impact_pct
            )?;
        }
        if matches!(
            entry.category,
            CleanupCategory::Unsupported | CleanupCategory::Burnable
        ) {
            writeln!(out, "  {}", safe(&entry.reason))?;
        }
    }
    for unknown in &plan.unparsed_accounts {
        writeln!(
            out,
            "SKIP  account={}  undecodable: {}",
            unknown.address,
            safe(&unknown.reason)
        )?;
    }
    writeln!(
        out,
        "\nEstimates exclude transaction fees. BURN permanently destroys the listed balances.\nNo transactions submitted."
    )
}
pub fn write_report(mut out: impl Write, report: &CleanupReport) -> io::Result<()> {
    let delta = report.known_swap_net_lamports;
    writeln!(
        out,
        "Cleanup results\nClosed: {}\nFailed/uncertain: {}\nSkipped: {}\nKnown swap wallet change (net fees/rent): {}{} SOL\nKnown reclaimed lamports: {}\nAccounting complete: {}",
        report.closed,
        report.failed,
        report.skipped,
        if delta < 0 { "-" } else { "" },
        exact_amount(delta.unsigned_abs(), 9),
        report.known_reclaimed_lamports,
        report.accounting_complete
    )?;
    for result in &report.results {
        writeln!(
            out,
            "{}  account={}  mint={}  {}",
            result.status,
            result.token_account,
            result.mint,
            safe(&result.reason)
        )?;
        for tx in &result.operations {
            writeln!(out, "  {:?}: {}", tx.operation, tx.signature)?;
        }
        if let Some(signature) = &result.uncertain_signature {
            writeln!(out, "  unresolved signature: {signature}")?;
        }
    }
    Ok(())
}
