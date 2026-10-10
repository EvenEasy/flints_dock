//! Synthetic category observations compile only in core tests and cannot affect production analysis.
#[cfg(test)]
use crate::core::{categories::*, *};
#[cfg(test)]
use serde::Deserialize;

pub const DEVNET_GENESIS: &str = "EtWTRABZaYq6iMfeYKouRu166VU2xqa1wcaWoxPkrZBG";

/// Labels and valuations are test observations; ownership and NFT evidence always come from RPC.
#[cfg(test)]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TestManifest {
    pub network: String,
    pub label: String,
    pub observations: Vec<TestObservation>,
}
#[cfg(test)]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TestObservation {
    pub mint: String,
    pub program: String,
    pub suspicious: Option<bool>,
    pub unit_usd: Option<f64>,
    pub routing: Option<TestRoute>,
}
#[cfg(test)]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TestRoute {
    Route,
    NoRoute,
    Unknown,
}

#[cfg(test)]
impl TestManifest {
    /// Reject wrong networks, invalid addresses, duplicates and invalid valuations at load time.
    pub fn parse(text: &str) -> Result<Self, String> {
        let value: Self =
            serde_json::from_str(text).map_err(|_| "Invalid test observation manifest")?;
        if value.network != DEVNET_GENESIS || value.label.trim().is_empty() {
            return Err("Test observations require devnet genesis and a dataset label".into());
        }
        let mut seen = std::collections::BTreeSet::new();
        for item in &value.observations {
            item.mint
                .parse::<solana_pubkey::Pubkey>()
                .map_err(|_| "Invalid test mint")?;
            if ![TokenProgram::Legacy, TokenProgram::Token2022]
                .iter()
                .any(|p| p.id().to_string() == item.program)
                || !seen.insert((&item.mint, &item.program))
                || item.unit_usd.is_some_and(|v| !v.is_finite() || v < 0.0)
            {
                return Err("Invalid program, duplicate holding or test valuation".into());
            }
        }
        Ok(value)
    }

    /// Enrich categories only. No mock quote or no-route observation reaches cleanup planning.
    pub fn apply(&self, report: &mut WalletCategories, snapshot: &WalletSnapshot) {
        if report.network != DEVNET_GENESIS || self.network != report.network {
            return;
        }
        let stamp = now().to_string();
        let source = format!(
            "TEST DATA: {}; devnet observations, not market quotes",
            self.label
        );
        for key in ["scam", "dust", "dead_token"] {
            let category = report.categories.get_mut(key).expect("standard category");
            category.items.clear();
            category.status = ScanStatus::Partial("Test observations cover only manifest mint/program pairs; other holdings are unknown".into());
            category.checked_at = stamp.clone();
        }
        let holdings = crate::core::amount::aggregate_holdings(if snapshot.selected.all_tokens {
            &snapshot.all_tokens
        } else {
            &snapshot.tokens
        });
        for holding in holdings
            .iter()
            .filter(|h| h.kind.is_fungible() && h.total_raw_amount > 0)
        {
            let Some(observation) = self
                .observations
                .iter()
                .find(|o| o.mint == holding.mint && o.program == holding.program.id().to_string())
            else {
                continue;
            };
            let value_usd = observation
                .unit_usd
                .zip(holding.decimals)
                .map(|(price, decimals)| {
                    price * holding.total_raw_amount as f64 / 10f64.powi(i32::from(decimals))
                })
                .filter(|v| v.is_finite());
            let item = CategoryItem {
                id: format!("{}:{}", holding.mint, holding.program.id()),
                mint: Some(holding.mint.clone()),
                program: Some(holding.program.id().to_string()),
                name: holding
                    .metadata
                    .name
                    .clone()
                    .unwrap_or_else(|| holding.mint.clone()),
                kind: "fungible".into(),
                accounts: holding.accounts.clone(),
                raw_amount: Some(holding.total_raw_amount.to_string()),
                risk: Some(RiskSignal {
                    status: if observation.suspicious == Some(true) {
                        "suspicious"
                    } else {
                        "unknown"
                    }
                    .into(),
                    source: source.clone(),
                    reasons: vec![
                        "Explicitly configured test observation; not a production risk verdict"
                            .into(),
                    ],
                    checked_at: stamp.clone(),
                }),
                valuation: if value_usd.is_some() {
                    "test_priced"
                } else {
                    "unpriced"
                }
                .into(),
                value_usd,
                tradability: match observation.routing {
                    Some(TestRoute::NoRoute) => "test_no_route",
                    Some(TestRoute::Route) => "test_route",
                    _ => "unknown",
                }
                .into(),
                evidence: Some(source.clone()),
                checked_at: stamp.clone(),
                provider_scope: Some(source.clone()),
            };
            if observation.suspicious == Some(true) {
                report
                    .categories
                    .get_mut("scam")
                    .unwrap()
                    .items
                    .push(item.clone());
            }
            if value_usd.is_some_and(|v| v > 0.0 && v <= report.dust_threshold_usd) {
                report
                    .categories
                    .get_mut("dust")
                    .unwrap()
                    .items
                    .push(item.clone());
            }
            if matches!(observation.routing, Some(TestRoute::NoRoute)) {
                report
                    .categories
                    .get_mut("dead_token")
                    .unwrap()
                    .items
                    .push(item);
            }
        }
        report
            .providers
            .insert("test_data".into(), ScanStatus::Partial(source));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_manifests_reject_mainnet_and_invalid_valuation() {
        assert!(
            TestManifest::parse(r#"{"network":"mainnet","label":"fixture","observations":[]}"#)
                .is_err()
        );
        let value = serde_json::json!({"network":DEVNET_GENESIS,"label":"fixture","observations":[{"mint":"11111111111111111111111111111111","program":TokenProgram::Legacy.id().to_string(),"unitUsd":-1}]});
        assert!(TestManifest::parse(&value.to_string()).is_err());
        assert!(
            TestManifest::parse(
                &serde_json::json!({"network":DEVNET_GENESIS,"label":"fixture","observations":[]})
                    .to_string()
            )
            .is_ok()
        );
    }
}
