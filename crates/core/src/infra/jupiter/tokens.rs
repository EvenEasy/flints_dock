use super::Jupiter;
use crate::{
    app::categories::RiskProvider,
    core::{ScanStatus, categories::*},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    time::Duration,
};

impl RiskProvider for Jupiter {
    async fn risks(&self, mints: &[String]) -> (BTreeMap<String, RiskSignal>, ScanStatus) {
        let unique: Vec<_> = mints
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let mut signals = BTreeMap::new();
        let mut failures = Vec::new();
        for batch in unique.chunks(100) {
            match self
                .mint_json("/tokens/v2/search", "query", batch, Duration::from_secs(60))
                .await
            {
                Ok(value) if value.is_array() => {
                    for token in value.as_array().unwrap() {
                        if let Some(mint) = token["id"]
                            .as_str()
                            .filter(|id| batch.iter().any(|mint| mint == id))
                        {
                            let flag = suspicious_flag(token);
                            signals.insert(
                                mint.into(),
                                RiskSignal {
                                    status: if flag == Some(true) {
                                        "suspicious"
                                    } else if flag == Some(false) {
                                        "not_flagged"
                                    } else {
                                        "unknown"
                                    }
                                    .into(),
                                    source: "Jupiter Tokens V2 audit.isSus".into(),
                                    reasons: if flag == Some(true) {
                                        vec![
                                            "Provider-flagged suspicious; not proof of fraud"
                                                .into(),
                                        ]
                                    } else if flag.is_none() {
                                        vec!["Jupiter did not supply an explicit suspicious audit flag; risk remains unknown".into()]
                                    } else {
                                        vec![]
                                    },
                                    checked_at: token["_checked_at"]
                                        .as_str()
                                        .unwrap_or("unknown")
                                        .to_string(),
                                },
                            );
                        }
                    }
                }
                _ => failures.push("Jupiter risk lookup unavailable or malformed"),
            }
        }
        let unknown = unique
            .iter()
            .filter(|id| signals.get(*id).is_none_or(|s| s.status == "unknown"))
            .count();
        let status = if failures.is_empty() && unknown == 0 {
            ScanStatus::Complete
        } else {
            ScanStatus::Partial(format!(
                "{unknown} unknown risk records; {} failed batches",
                failures.len()
            ))
        };
        (signals, status)
    }
}

fn suspicious_flag(token: &serde_json::Value) -> Option<bool> {
    token["audit"]["isSus"].as_bool()
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn only_explicit_provider_flag_establishes_suspicion() {
        for token in [
            json!({}),
            json!({"isVerified":false,"organicScore":0,"audit":{"freezeAuthorityDisabled":false,"mintAuthorityDisabled":false}}),
            json!({"audit":{"isSus":"true"}}),
        ] {
            assert_eq!(suspicious_flag(&token), None);
        }
        assert_eq!(
            suspicious_flag(&json!({"audit":{"isSus":true}})),
            Some(true)
        );
        assert_eq!(
            suspicious_flag(&json!({"audit":{"isSus":false}})),
            Some(false)
        );
    }
}
