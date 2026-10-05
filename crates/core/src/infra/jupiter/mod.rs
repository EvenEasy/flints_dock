mod price;
pub mod swap;
pub use price::parse_quote;
use std::time::Duration;

pub use crate::core::asset::WRAPPED_SOL;

/// Shared transport and authentication for Jupiter Price V3 and Swap V2.
/// Changing the Solana RPC URL does not change this adapter's API endpoint or network.
pub struct Jupiter {
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
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(20))
                .redirect(reqwest::redirect::Policy::none())
                .build()?,
            api_key,
            base_url,
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
