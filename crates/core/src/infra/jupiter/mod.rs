mod price;
pub mod swap;
mod tokens;
pub use price::parse_quote;
use std::time::Duration;

pub use crate::core::asset::WRAPPED_SOL;

/// Shared transport and authentication for Jupiter Price V3 and Swap V2.
/// Changing the Solana RPC URL does not change this adapter's API endpoint or network.
#[derive(Clone)]
pub struct Jupiter {
    gate: std::sync::Arc<std::sync::Mutex<std::time::Instant>>,
    cache: std::sync::Arc<
        std::sync::Mutex<
            std::collections::BTreeMap<String, (std::time::Instant, serde_json::Value)>,
        >,
    >,
    http: reqwest::Client,
    api_key: reqwest::header::HeaderValue,
    base_url: String,
}
impl Jupiter {
    pub fn new(api_key: String) -> anyhow::Result<Self> {
        Self::with_base_url(api_key, "https://api.jup.ag".into())
    }
    fn with_base_url(api_key: String, base_url: String) -> anyhow::Result<Self> {
        let mut api_key = reqwest::header::HeaderValue::from_str(&api_key)?;
        api_key.set_sensitive(true);
        Ok(Self {
            gate: std::sync::Arc::new(std::sync::Mutex::new(std::time::Instant::now())),
            cache: Default::default(),
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(20))
                .redirect(reqwest::redirect::Policy::none())
                .build()?,
            api_key,
            base_url,
        })
    }

    /// All Price/Tokens/Swap requests share pacing and bounded provider-directed backoff.
    async fn json_get(
        &self,
        path: &str,
        params: &[(&str, String)],
        ttl: Duration,
    ) -> Result<(u16, serde_json::Value), String> {
        let key = format!("mainnet:{path}:{params:?}");
        if !ttl.is_zero()
            && let Some((at, value)) = self
                .cache
                .lock()
                .map_err(|_| "cache unavailable")?
                .get(&key)
            && at.elapsed() < ttl
        {
            return Ok((200, value.clone()));
        }
        for attempt in 0..3 {
            let delay = {
                let mut gate = self.gate.lock().map_err(|_| "limiter unavailable")?;
                let now = std::time::Instant::now();
                let at = (*gate).max(now);
                *gate =
                    at + Duration::from_millis(if self.api_key.is_empty() { 2100 } else { 1100 });
                at.saturating_duration_since(now)
            };
            tokio::time::sleep(delay).await;
            let response = self
                .get(path)
                .query(params)
                .send()
                .await
                .map_err(|e| e.without_url().to_string());
            let response = match response {
                Ok(response) => response,
                Err(_) if attempt < 2 => {
                    tokio::time::sleep(Duration::from_secs(1 << attempt)).await;
                    continue;
                }
                Err(error) => return Err(error),
            };
            let status = response.status().as_u16();
            let headers = response.headers();
            let retry_after = headers
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok());
            let reset = headers
                .get("x-ratelimit-reset")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok())
                .map(|t| t.saturating_sub(crate::core::categories::now()));
            if headers
                .get("x-ratelimit-remaining")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<i64>().ok())
                .is_some_and(|v| v <= 0)
            {
                let mut gate = self.gate.lock().map_err(|_| "limiter unavailable")?;
                *gate = (*gate).max(
                    std::time::Instant::now() + Duration::from_secs(reset.unwrap_or(2).min(60)),
                );
            }
            let value: serde_json::Value = response
                .json()
                .await
                .map_err(|e| e.without_url().to_string())?;
            if (status == 429 || status >= 500) && attempt < 2 {
                let wait = retry_after.or(reset).unwrap_or(1 << attempt);
                if wait > 60 {
                    return Ok((status, value));
                }
                tokio::time::sleep(Duration::from_secs(wait)).await;
                continue;
            }
            if status == 200 && !ttl.is_zero() {
                let mut cache = self.cache.lock().map_err(|_| "cache unavailable")?;
                cache.retain(|_, (at, _)| at.elapsed() < Duration::from_secs(300));
                if cache.len() >= 256 {
                    cache.clear();
                }
                cache.insert(key, (std::time::Instant::now(), value.clone()));
            }
            return Ok((status, value));
        }
        Err("Jupiter retries exhausted".into())
    }
    /// Cache exact-mint observations, so changing wallet/batch composition still reuses fresh data.
    async fn mint_json(
        &self,
        path: &str,
        parameter: &str,
        mints: &[String],
        ttl: Duration,
    ) -> Result<serde_json::Value, String> {
        let mut known = serde_json::Map::new();
        let mut missing = Vec::new();
        {
            let cache = self.cache.lock().map_err(|_| "cache unavailable")?;
            for mint in mints {
                let key = format!("mainnet:{path}:mint:{mint}");
                match cache.get(&key).filter(|(at, _)| at.elapsed() < ttl) {
                    Some((_, value)) => {
                        known.insert(mint.clone(), value.clone());
                    }
                    None => missing.push(mint.clone()),
                }
            }
        }
        if !missing.is_empty() {
            let (status, value) = self
                .json_get(path, &[(parameter, missing.join(","))], Duration::ZERO)
                .await?;
            if status != 200 {
                return Err(format!("Jupiter HTTP {status}"));
            }
            let mut fetched = if path == "/tokens/v2/search" {
                let Some(rows) = value.as_array() else {
                    return Err("Malformed Jupiter token response".into());
                };
                rows.iter()
                    .filter_map(|row| row["id"].as_str().map(|id| (id.to_owned(), row.clone())))
                    .collect::<serde_json::Map<_, _>>()
            } else {
                value
                    .as_object()
                    .cloned()
                    .ok_or("Malformed Jupiter price response")?
            };
            let mut cache = self.cache.lock().map_err(|_| "cache unavailable")?;
            cache.retain(|_, (at, _)| at.elapsed() < Duration::from_secs(300));
            if cache.len() > 4096 {
                cache.clear();
            }
            for mint in missing {
                let mut row = fetched.remove(&mint).unwrap_or(serde_json::Value::Null);
                if let Some(row) = row.as_object_mut() {
                    row.insert(
                        "_checked_at".into(),
                        crate::core::categories::now().to_string().into(),
                    );
                }
                cache.insert(
                    format!("mainnet:{path}:mint:{mint}"),
                    (std::time::Instant::now(), row.clone()),
                );
                known.insert(mint, row);
            }
        }
        Ok(if path == "/tokens/v2/search" {
            serde_json::Value::Array(known.into_values().filter(|v| !v.is_null()).collect())
        } else {
            serde_json::Value::Object(known)
        })
    }

    fn get(&self, path: &str) -> reqwest::RequestBuilder {
        let request = self.http.get(format!("{}{path}", self.base_url));
        if self.api_key.is_empty() {
            request
        } else {
            request.header("x-api-key", self.api_key.clone())
        }
    }
}

#[cfg(test)]
mod tests;
