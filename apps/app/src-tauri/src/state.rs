use crate::error::AppError;
use dock_flints_core::infra::wallet::WalletIdentity;
use dock_flints_core::infra::{jupiter::Jupiter, solana};
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use std::sync::Mutex;

/// Shared clients and an optional local identity; signer material never enters response DTOs.
pub struct AppState {
    pub(crate) rpc: RpcClient,
    pub(crate) identity: Mutex<Option<WalletIdentity>>,
    pub(crate) jupiter: Option<Jupiter>,
    pub(crate) pricing_error: Option<String>,
}

impl AppState {
    /// Read backend configuration from the environment, keeping clients out of IPC requests.
    /// Defaults match the CLI's mainnet endpoint and 30-second confirmed RPC timeout.
    pub fn from_env() -> Result<Self, AppError> {
        let url = std::env::var("DOCK_FLINTS_RPC_URL")
            .unwrap_or_else(|_| "https://api.mainnet.solana.com".into());
        let timeout = std::env::var("DOCK_FLINTS_RPC_TIMEOUT_SECONDS")
            .unwrap_or_else(|_| "30".into())
            .parse::<u64>()
            .map_err(|_| Self::timeout_error())?;
        let key = std::env::var("JUPITER_API_KEY")
            .ok()
            .filter(|key| !key.trim().is_empty());
        Self::new(url, timeout, key)
    }

    /// Validate transport settings and create the existing core clients without network reads.
    /// A malformed pricing key disables pricing rather than disabling wallet discovery.
    pub fn new(
        url: String,
        timeout_seconds: u64,
        api_key: Option<String>,
    ) -> Result<Self, AppError> {
        let parsed = reqwest::Url::parse(&url).map_err(|_| Self::url_error())?;
        if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
            return Err(Self::url_error());
        }
        if !(1..=300).contains(&timeout_seconds) {
            return Err(Self::timeout_error());
        }

        // Reuse the core adapter: an empty key enables Jupiter's lower-limit keyless access.
        // A configured key stays in Rust and is sent only through the existing sensitive header.
        let (jupiter, pricing_error) = match Jupiter::new(api_key.unwrap_or_default()) {
            Ok(provider) => (Some(provider), None),
            Err(_) => (
                None,
                Some(
                    "Jupiter pricing client could not be initialized; check JUPITER_API_KEY".into(),
                ),
            ),
        };
        Ok(Self {
            identity: Mutex::new(None),
            rpc: solana::client(url, timeout_seconds),
            jupiter,
            pricing_error,
        })
    }

    fn url_error() -> AppError {
        AppError::configuration(
            "DOCK_FLINTS_RPC_URL",
            "RPC URL must be an absolute HTTP(S) URL",
        )
    }

    fn timeout_error() -> AppError {
        AppError::configuration(
            "DOCK_FLINTS_RPC_TIMEOUT_SECONDS",
            "RPC timeout must be between 1 and 300 seconds",
        )
    }
}

#[cfg(test)]
mod tests {
    use super::AppState;

    #[test]
    fn jupiter_is_available_without_a_key_and_invalid_keys_do_not_disable_rpc() {
        // Keyless pricing must not silently become "No price provider configured".
        let keyless = AppState::new("http://127.0.0.1:8899".into(), 30, None).unwrap();
        assert!(keyless.jupiter.is_some());
        assert!(keyless.pricing_error.is_none());

        // Invalid authentication configuration is isolated from wallet discovery.
        let invalid = AppState::new(
            "http://127.0.0.1:8899".into(),
            30,
            Some("invalid\nheader".into()),
        )
        .unwrap();
        assert!(invalid.jupiter.is_none());
        assert!(invalid.pricing_error.unwrap().contains("JUPITER_API_KEY"));
    }
}
