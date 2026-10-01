use super::{amount::exact_amount, error::*};
use serde::Serialize;
use solana_pubkey::Pubkey;
use std::time::{Duration, Instant};
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwapRequest {
    pub mint: Pubkey,
    pub raw_amount: u64,
    pub wallet: Pubkey,
    pub slippage_bps: u16,
}
impl SwapRequest {
    pub fn validate(&self) -> Result<()> {
        if self.raw_amount == 0 || self.mint.to_string() == crate::core::asset::WRAPPED_SOL {
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
    #[serde(serialize_with = "crate::core::integer_string")]
    pub expected_out_lamports: u64,
    #[serde(serialize_with = "crate::core::integer_string")]
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
    pub build: SwapTransaction,
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

/// A preview is bound to the exact mint, amount, wallet and slippage the user saw.
pub struct SwapPreview {
    pub(crate) request: SwapRequest,
    pub quote: SwapQuote,
}
#[derive(Debug, Serialize)]
pub struct SwapReceipt {
    pub status: &'static str,
    pub signature: String,
    pub output_asset: &'static str,
    pub fresh_quote: SwapQuote,
}

/// Provider-independent Solana transaction ingredients. Wire decoding belongs to
/// the adapter; signing, simulation and fee limits belong to the chain executor.
/// These are chain data types, not RPC clients or provider response objects.
pub struct SwapTransaction {
    pub compute_budget_instructions: Vec<solana_instruction::Instruction>,
    pub setup_instructions: Vec<solana_instruction::Instruction>,
    pub swap_instruction: solana_instruction::Instruction,
    pub cleanup_instruction: Option<solana_instruction::Instruction>,
    pub other_instructions: Vec<solana_instruction::Instruction>,
    pub tip_instruction: Option<solana_instruction::Instruction>,
    pub lookup_tables: Vec<solana_message::AddressLookupTableAccount>,
    pub blockhash: solana_hash::Hash,
    pub last_valid_block_height: u64,
}
