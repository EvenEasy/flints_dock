use crate::{
    cli::output::{OutputOptions, nft_rows, token_name, visible_token_assets},
    core::amount::exact_amount,
    core::*,
};
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

pub fn money(value: Option<f64>) -> String {
    match value.filter(|value| value.is_finite() && *value >= 0.0) {
        None => "-".into(),
        Some(0.0) => "$0.00".into(),
        Some(value) if value >= 0.01 => format!("${value:.2}"),
        Some(value) if value >= 0.000001 => format!("${value:.8}").trim_end_matches('0').to_owned(),
        Some(value) => format!("${value:.3e}"),
    }
}

fn label(value: &str) -> String {
    // Preserve Unicode names, but never let on-chain text execute terminal controls.
    value
        .chars()
        .flat_map(|ch| {
            if ch.is_control() {
                ch.escape_default().collect::<Vec<_>>()
            } else {
                vec![ch]
            }
        })
        .collect()
}
fn name(value: &str) -> String {
    let safe = label(value);
    if safe.chars().count() > 32 {
        format!("{}…", safe.chars().take(31).collect::<String>())
    } else {
        safe
    }
}
fn table(out: &mut impl Write, headings: Vec<String>, rows: Vec<Vec<String>>) -> io::Result<()> {
    let mut widths: Vec<_> = headings.iter().map(|cell| cell.chars().count()).collect();
    for row in &rows {
        for (width, cell) in widths.iter_mut().zip(row) {
            *width = (*width).max(cell.chars().count());
        }
    }
    for row in std::iter::once(headings).chain(rows) {
        write!(out, "  ")?;
        for (i, cell) in row.iter().enumerate() {
            write!(out, "{cell}")?;
            if i + 1 < row.len() {
                write!(out, "{}", " ".repeat(widths[i] - cell.chars().count() + 2))?;
            }
        }
        writeln!(out)?;
    }
    Ok(())
}

fn heading(
    out: &mut impl Write,
    title: &str,
    count: usize,
    scan: Option<&ScanStatus>,
) -> io::Result<bool> {
    match scan {
        Some(ScanStatus::Complete) => {
            if count == 0 {
                writeln!(out, "\n{title}: none")?;
            } else {
                writeln!(out, "\n{title} ({count})")?;
            }
        }
        Some(ScanStatus::Partial(_)) => {
            writeln!(out, "\n{title} ({count} found; incomplete)")?;
        }
        _ => {
            writeln!(out, "\n{title}: unavailable")?;
        }
    }
    Ok(count > 0)
}

pub fn write_portfolio(
    mut out: impl Write,
    portfolio: &WalletSnapshot,
    options: &OutputOptions,
) -> io::Result<()> {
    let wallet = &portfolio.owner;
    let shortened = if wallet.len() > 16 {
        format!("{}...{}", &wallet[..8], &wallet[wallet.len() - 5..])
    } else {
        wallet.clone()
    };
    writeln!(
        out,
        "Wallet: {}",
        if options.details { wallet } else { &shortened }
    )?;
    if portfolio.selected.balance {
        if let Some(native) = &portfolio.native_sol {
            writeln!(
                out,
                "\nSOL\n  Balance: {} SOL ({})",
                exact_amount(native.lamports.into(), 9),
                money(native.value_usd)
            )?;
            if options.show_price {
                writeln!(
                    out,
                    "  Price: {}",
                    money(native.price.as_ref().map(|price| price.usd))
                )?;
            }
            if options.details {
                writeln!(out, "  Lamports: {}", native.lamports)?;
            }
        } else {
            heading(&mut out, "SOL", 0, portfolio.scanners.get("native_sol"))?;
        }
    }
    for (selected, key, title, assets) in [
        (
            portfolio.selected.tokens,
            "tokens",
            "TOKENS",
            &portfolio.tokens,
        ),
        (
            portfolio.selected.all_tokens,
            "all_tokens",
            "ALL TOKEN ASSETS",
            &portfolio.all_tokens,
        ),
    ] {
        if !selected {
            continue;
        }
        let tokens = visible_token_assets(assets, options);
        if heading(&mut out, title, tokens.len(), portfolio.scanners.get(key))? {
            let mut headers = vec![
                "Token".into(),
                "Balance".into(),
                "Type".into(),
                "Value".into(),
            ];
            if options.show_price {
                headers.push("Price".into());
            }
            if options.show_mint || options.details {
                headers.push("Mint".into());
            }
            if options.details {
                headers.extend(
                    [
                        "Raw",
                        "Decimals",
                        "Program",
                        "Accounts",
                        "Lamports",
                        "Metadata URI",
                    ]
                    .map(str::to_owned),
                );
            }
            let rows = tokens
                .iter()
                .map(|token| {
                    let mut row = vec![
                        name(token_name(token)),
                        token.balance.clone().unwrap_or_else(|| "-".into()),
                        token.kind.label().into(),
                        money(token.value_usd),
                    ];
                    if options.show_price {
                        row.push(money(token.price.as_ref().map(|price| price.usd)));
                    }
                    if options.show_mint || options.details {
                        row.push(token.mint.clone());
                    }
                    if options.details {
                        let lamports: u128 = portfolio
                            .token_accounts
                            .iter()
                            .filter(|account| token.accounts.contains(&account.address))
                            .map(|account| u128::from(account.lamports))
                            .sum();
                        row.extend([
                            token.total_raw_amount.to_string(),
                            token
                                .decimals
                                .map(|decimals| decimals.to_string())
                                .unwrap_or_else(|| "-".into()),
                            format!("{:?}", token.program),
                            token.accounts.join(","),
                            lamports.to_string(),
                            label(token.metadata.uri.as_deref().unwrap_or("-")),
                        ]);
                    }
                    row
                })
                .collect();
            table(&mut out, headers, rows)?;
        }
    }
    if portfolio.selected.nfts {
        let nfts = nft_rows(portfolio);
        if heading(&mut out, "NFTs", nfts.len(), portfolio.scanners.get("nfts"))? {
            let mut headers = vec!["Name".into(), "Type".into()];
            if options.show_mint {
                headers.push("Asset ID".into());
            }
            if options.details {
                headers.extend(
                    ["Collection", "Metadata URI", "Accounts", "Lamports"].map(str::to_owned),
                );
            }
            let rows = nfts
                .iter()
                .map(|nft| {
                    let mut row = vec![name(nft.name), nft.kind.into()];
                    if options.show_mint {
                        row.push(nft.asset_id.into());
                    }
                    if options.details {
                        let collection = nft
                            .collection
                            .map(|collection| {
                                format!(
                                    "{}{}",
                                    collection.address,
                                    if collection.verified {
                                        ""
                                    } else {
                                        " (unverified)"
                                    }
                                )
                            })
                            .unwrap_or_else(|| "-".into());
                        row.extend([
                            collection,
                            label(nft.uri.unwrap_or("-")),
                            nft.accounts.join(","),
                            nft.lamports.to_string(),
                        ]);
                    }
                    row
                })
                .collect();
            table(&mut out, headers, rows)?;
        }
    }
    if portfolio.selected.cnfts {
        writeln!(out, "\ncNFTs: unavailable (requires historical index)")?;
    }
    if let Some(scan) = portfolio.scanners.get("prices") {
        match scan {
            ScanStatus::Skipped(reason) if reason == "No price provider configured" => {
                writeln!(out, "\nPrices: unavailable (set JUPITER_API_KEY)")?
            }
            ScanStatus::Failed(_) => {
                writeln!(out, "\nPrices: unavailable (provider request failed)")?
            }
            _ => {}
        }
    }
    Ok(())
}
