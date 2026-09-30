use crate::{
    models::*,
    output::{OutputOptions, nft_rows, visible_token_assets},
    portfolio::aggregate::exact_amount,
};
use serde_json::{Map, Value, json};

fn category(status: Option<&ScanStatus>) -> Map<String, Value> {
    match status.and_then(|status| serde_json::to_value(status).ok()) {
        Some(Value::Object(map)) => map,
        _ => Map::from_iter([
            ("status".into(), json!("failed")),
            ("reason".into(), json!("Scanner did not run")),
        ]),
    }
}

/// Presentation projection: shared token discovery must not expose unrequested categories.
pub fn portfolio_json(portfolio: &Portfolio, options: &OutputOptions) -> Value {
    let mut output = Map::from_iter([("wallet".into(), json!(portfolio.owner))]);
    if portfolio.selected.balance {
        let mut sol = category(portfolio.scanners.get("native_sol"));
        if let Some(native) = &portfolio.native_sol {
            sol.insert("lamports".into(), json!(native.lamports));
            sol.insert(
                "balance".into(),
                json!(exact_amount(native.lamports.into(), 9)),
            );
            sol.insert("usd_value".into(), json!(native.value_usd));
            sol.insert("price".into(), json!(native.price));
        }
        sol.insert("pricing".into(), json!(portfolio.scanners.get("prices")));
        output.insert("sol".into(), Value::Object(sol));
    }
    for (selected, key, assets) in [
        (portfolio.selected.tokens, "tokens", &portfolio.tokens),
        (
            portfolio.selected.all_tokens,
            "all_tokens",
            &portfolio.all_tokens,
        ),
    ] {
        if !selected {
            continue;
        }
        let mut tokens = category(portfolio.scanners.get(key));
        tokens.insert("items".into(), json!(visible_token_assets(assets, options)));
        tokens.insert("pricing".into(), json!(portfolio.scanners.get("prices")));
        tokens.insert(
            "unknown".into(),
            json!(
                portfolio
                    .unknown_assets
                    .iter()
                    .filter(|asset| asset.program_id.as_deref()
                        != Some(mpl_core::ID.to_string().as_str()))
                    .collect::<Vec<_>>()
            ),
        );
        if options.details {
            // Retain the shared raw inventory, including empty accounts, for cleanup analysis.
            tokens.insert(
                "discovered_accounts".into(),
                json!(portfolio.token_accounts),
            );
            tokens.insert("mints".into(), json!(portfolio.mints));
            tokens.insert("account_summary".into(), json!(portfolio.account_summary));
        }
        output.insert(key.into(), Value::Object(tokens));
    }
    if portfolio.selected.nfts {
        let mut nfts = category(portfolio.scanners.get("nfts"));
        let items: Vec<_> = nft_rows(portfolio)
            .into_iter()
            .map(|nft| {
                json!({
                    "name": nft.name, "type": nft.kind, "asset_id": nft.asset_id,
                    "collection": nft.collection, "uri": nft.uri,
                })
            })
            .collect();
        nfts.insert("items".into(), json!(items));
        nfts.insert("sources".into(), json!({ "classic": portfolio.scanners.get("classic_nfts"), "core": portfolio.scanners.get("core_asset_v1") }));
        nfts.insert(
            "unknown".into(),
            json!(
                portfolio
                    .unknown_assets
                    .iter()
                    .filter(|asset| {
                        asset.program_id.as_deref() == Some(mpl_core::ID.to_string().as_str())
                            || portfolio.token_accounts.iter().any(|account| {
                                account.mint == asset.address
                                    && account.raw_amount == 1
                                    && account.decimals == Some(0)
                            })
                    })
                    .collect::<Vec<_>>()
            ),
        );
        if options.details {
            nfts.insert("classic_details".into(), json!(portfolio.classic_nfts));
            nfts.insert("core_details".into(), json!(portfolio.core_assets));
            let accounts: Vec<_> = portfolio
                .token_accounts
                .iter()
                .filter(|account| {
                    portfolio
                        .classic_nfts
                        .iter()
                        .any(|nft| nft.token_accounts.contains(&account.address))
                })
                .collect();
            nfts.insert("token_accounts".into(), json!(accounts));
        }
        output.insert("nfts".into(), Value::Object(nfts));
    }
    if portfolio.selected.cnfts {
        let mut cnfts = category(portfolio.scanners.get("compressed_nfts"));
        cnfts.insert("code".into(), json!("historical_index_required"));
        // Null explicitly means not enumerated; [] would imply a successful empty result.
        cnfts.insert("items".into(), Value::Null);
        output.insert("cnfts".into(), Value::Object(cnfts));
    }
    Value::Object(output)
}
