use crate::{models::*, pricing::PriceProvider};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    time::Duration,
};

pub const WRAPPED_SOL: &str = "So11111111111111111111111111111111111111112";
const ENDPOINT: &str = "https://api.jup.ag/price/v3";

pub struct Jupiter {
    http: reqwest::Client,
    api_key: String,
}
impl Jupiter {
    pub fn new(api_key: String) -> anyhow::Result<Self> {
        Ok(Self {
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(20))
                .build()?,
            api_key,
        })
    }
}

pub fn parse_quote(value: &Value) -> Option<Price> {
    let usd = value["usdPrice"].as_f64()?;
    if !usd.is_finite() || usd < 0.0 {
        return None;
    }
    Some(Price {
        usd,
        source: "Jupiter Price V3".into(),
        block_id: value["blockId"].as_u64(),
        decimals: value["decimals"].as_u64()?.try_into().ok()?,
    })
}

impl PriceProvider for Jupiter {
    async fn get_prices(&self, mints: &[String]) -> PriceReport {
        let unique: Vec<_> = mints
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let mut quotes = BTreeMap::new();
        let mut errors = Vec::new();
        let mut successes = 0;
        // Price V3 takes at most 50 IDs; each mint is requested once per run.
        for batch in unique.chunks(50) {
            let response = async {
                self.http
                    .get(ENDPOINT)
                    .header("x-api-key", &self.api_key)
                    .query(&[("ids", batch.join(","))])
                    .send()
                    .await?
                    .error_for_status()?
                    .json::<Value>()
                    .await
            }
            .await;
            match response {
                Ok(value) if value.is_object() => {
                    successes += 1;
                    for mint in batch {
                        if let Some(quote) = parse_quote(&value[mint]) {
                            quotes.insert(mint.clone(), quote);
                        }
                    }
                }
                Ok(_) => errors.push("Jupiter returned an unexpected response shape".to_owned()),
                Err(error) => errors.push(error.without_url().to_string()),
            }
        }
        let status = if !errors.is_empty() && successes == 0 {
            ScanStatus::Failed(errors.join("; "))
        } else if !errors.is_empty() || quotes.len() < unique.len() {
            ScanStatus::Partial(format!(
                "{}/{} mints priced; {}",
                quotes.len(),
                unique.len(),
                if errors.is_empty() {
                    "unquoted mints remain unvalued".into()
                } else {
                    errors.join("; ")
                }
            ))
        } else {
            ScanStatus::Complete
        };
        PriceReport { status, quotes }
    }
}
