use crate::{jupiter::swap::BuildResponse, portfolio::aggregate::exact_amount};
use serde::Serialize;
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use std::time::{Duration, Instant};

#[derive(Debug, thiserror::Error)]
pub enum SwapError {
    #[error("Invalid swap request: {0}")]
    InvalidRequest(String),
    #[error("No TOKEN/SOL route: {0}")]
    NoRoute(String),
    #[error("Insufficient liquidity: {0}")]
    InsufficientLiquidity(String),
    #[error("Insufficient funds: {0}")]
    InsufficientFunds(String),
    #[error("Jupiter request failed: {0}")]
    Api(String),
    #[error("Invalid Jupiter response: {0}")]
    InvalidResponse(String),
    #[error("Price impact {actual_pct:.4}% exceeds limit {max_pct:.4}%")]
    PriceImpact { actual_pct: f64, max_pct: f64 },
    #[error("Quote expired before submission; request a new preview")]
    Expired,
    #[error("Fresh quote is worse than the approved minimum; request a new preview")]
    PreviewChanged,
    #[error("RPC failed before submission: {0}")]
    Rpc(String),
    #[error("Simulation failed: {0}")]
    Simulation(String),
    #[error("Transaction failed on-chain ({signature}): {reason}")]
    TransactionFailed { signature: String, reason: String },
    #[error(
        "Submission/confirmation uncertain ({signature}): {reason}. Check this signature before retrying"
    )]
    Uncertain { signature: String, reason: String },
}
pub type Result<T> = std::result::Result<T, SwapError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwapRequest {
    pub mint: Pubkey,
    pub raw_amount: u64,
    pub wallet: Pubkey,
    pub slippage_bps: u16,
}
impl SwapRequest {
    pub fn validate(&self) -> Result<()> {
        if self.raw_amount == 0 || self.mint.to_string() == crate::jupiter::WRAPPED_SOL {
            return Err(SwapError::InvalidRequest(
                "amount must be positive and input mint must differ from WSOL".into(),
            ));
        }
        if self.slippage_bps == 0 || self.slippage_bps > 10_000 {
            return Err(SwapError::InvalidRequest(
                "slippage must be 1..=10000 basis points".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct SwapLimits {
    pub max_price_impact_bps: u16,
    pub max_quote_age: Duration,
    pub max_priority_fee_lamports: u64,
    pub confirmation_timeout: Duration,
}
impl Default for SwapLimits {
    fn default() -> Self {
        Self {
            max_price_impact_bps: 100,
            max_quote_age: Duration::from_secs(30),
            max_priority_fee_lamports: 1_000_000,
            confirmation_timeout: Duration::from_secs(90),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SwapQuote {
    pub input_mint: String,
    pub output_mint: String,
    pub raw_amount: String,
    #[serde(serialize_with = "crate::models::integer_string")]
    pub expected_out_lamports: u64,
    #[serde(serialize_with = "crate::models::integer_string")]
    pub min_out_lamports: u64,
    pub expected_out_sol: String,
    pub min_out_sol: String,
    pub price_impact_pct: f64,
    pub slippage_bps: u16,
    pub route_exists: bool,
    pub route_labels: Vec<String>,
}
impl SwapQuote {
    pub(crate) fn amounts(mut self) -> Self {
        self.expected_out_sol = exact_amount(self.expected_out_lamports.into(), 9);
        self.min_out_sol = exact_amount(self.min_out_lamports.into(), 9);
        self
    }
    pub fn check_impact(&self, limits: &SwapLimits) -> Result<()> {
        if self.price_impact_pct.abs() > f64::from(limits.max_price_impact_bps) / 100.0 {
            return Err(SwapError::PriceImpact {
                actual_pct: self.price_impact_pct,
                max_pct: f64::from(limits.max_price_impact_bps) / 100.0,
            });
        }
        Ok(())
    }
}

pub struct PreparedSwap {
    pub request: SwapRequest,
    pub quote: SwapQuote,
    pub build: BuildResponse,
    pub requested_at: Instant,
}
impl PreparedSwap {
    pub fn ensure_fresh(&self, limits: &SwapLimits) -> Result<()> {
        if self.requested_at.elapsed() >= limits.max_quote_age {
            Err(SwapError::Expired)
        } else {
            Ok(())
        }
    }
}

pub trait SwapProvider {
    fn build_swap(
        &self,
        request: &SwapRequest,
    ) -> impl std::future::Future<Output = Result<PreparedSwap>> + Send;
}
pub trait SwapExecutor {
    fn check_input(
        &self,
        request: &SwapRequest,
    ) -> impl std::future::Future<Output = Result<()>> + Send;
    fn submit(
        &self,
        fresh: PreparedSwap,
        signer: &solana_keypair::Keypair,
        limits: &SwapLimits,
    ) -> impl std::future::Future<Output = Result<String>> + Send;
}

/// A preview is bound to the exact mint, amount, wallet and slippage the user saw.
pub struct SwapPreview {
    request: SwapRequest,
    pub quote: SwapQuote,
}
#[derive(Debug, Serialize)]
pub struct SwapReceipt {
    pub status: &'static str,
    pub signature: String,
    pub output_asset: &'static str,
    pub fresh_quote: SwapQuote,
}

pub async fn preview(
    provider: &impl SwapProvider,
    request: &SwapRequest,
    limits: &SwapLimits,
) -> Result<SwapPreview> {
    request.validate()?;
    let built = provider.build_swap(request).await?;
    built.ensure_fresh(limits)?;
    built.quote.check_impact(limits)?;
    Ok(SwapPreview {
        request: request.clone(),
        quote: built.quote,
    })
}

/// Never accepts preview instructions: every execution builds a new route/transaction.
/// There is no retry that could rebuild and submit a second economic transaction.
pub async fn execute(
    provider: &impl SwapProvider,
    executor: &impl SwapExecutor,
    approved: &SwapPreview,
    signer: &solana_keypair::Keypair,
    limits: &SwapLimits,
) -> Result<SwapReceipt> {
    if signer.pubkey() != approved.request.wallet {
        return Err(SwapError::InvalidRequest(
            "keypair does not match preview wallet".into(),
        ));
    }
    executor.check_input(&approved.request).await?;
    let fresh = provider.build_swap(&approved.request).await?;
    fresh.ensure_fresh(limits)?;
    fresh.quote.check_impact(limits)?;
    if fresh.quote.min_out_lamports < approved.quote.min_out_lamports {
        return Err(SwapError::PreviewChanged);
    }
    let quote = fresh.quote.clone();
    let signature = executor.submit(fresh, signer, limits).await?;
    Ok(SwapReceipt {
        status: "confirmed",
        signature,
        output_asset: "native SOL",
        fresh_quote: quote,
    })
}
