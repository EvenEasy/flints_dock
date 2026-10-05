use super::{Jupiter, WRAPPED_SOL};
use crate::app::swap::{PreparedSwap, Result, SwapError, SwapProvider, SwapQuote, SwapRequest};
use serde::Deserialize;
use std::{collections::BTreeMap, time::Instant};

/// Decode a Jupiter instruction account address and signer/writable flags.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiAccount {
    pub pubkey: String,
    pub is_signer: bool,
    pub is_writable: bool,
}

/// Keep Jupiter wire instruction fields inside the provider adapter.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiInstruction {
    pub program_id: String,
    pub accounts: Vec<ApiAccount>,
    pub data: String,
}

/// Decode the recent blockhash and its last valid block height.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockhashMetadata {
    pub blockhash: [u8; 32],
    pub last_valid_block_height: u64,
}

/// Decode one step of the provider-selected swap route.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteStep {
    pub swap_info: RouteInfo,
}

/// Retain the optional route label supplied by the provider.
#[derive(Debug, Clone, Deserialize)]
pub struct RouteInfo {
    pub label: Option<String>,
}

/// Deserialize the Jupiter build response before mapping it into core types.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildResponse {
    pub input_mint: String,
    pub output_mint: String,
    pub in_amount: String,
    pub out_amount: String,
    pub other_amount_threshold: String,
    pub swap_mode: String,
    pub slippage_bps: u16,
    pub price_impact_pct: String,
    pub route_plan: Vec<RouteStep>,
    pub compute_budget_instructions: Vec<ApiInstruction>,
    pub setup_instructions: Vec<ApiInstruction>,
    pub swap_instruction: ApiInstruction,
    pub cleanup_instruction: Option<ApiInstruction>,
    pub other_instructions: Vec<ApiInstruction>,
    pub tip_instruction: Option<ApiInstruction>,
    pub addresses_by_lookup_table_address: Option<BTreeMap<String, Vec<String>>>,
    pub blockhash_with_metadata: BlockhashMetadata,
}
impl BuildResponse {
    /// Return a normalized quote after validating the provider response against `request`.
    /// Checks mint identity, exact input, slippage floor and finite impact; returns distinct
    /// no-route, liquidity and invalid-response errors.
    pub fn quote(&self, request: &SwapRequest) -> Result<SwapQuote> {
        request.validate()?;

        // Reject responses for a different mint, amount, output asset or slippage policy.
        if self.input_mint != request.mint.to_string()
            || self.output_mint != WRAPPED_SOL
            || self.in_amount.parse::<u64>().ok() != Some(request.raw_amount)
            || self.swap_mode != "ExactIn"
            || self.slippage_bps != request.slippage_bps
        {
            return Err(SwapError::InvalidResponse(
                "mint, amount, mode or slippage mismatch".into(),
            ));
        }
        if self.route_plan.is_empty() {
            return Err(SwapError::NoRoute("empty route plan".into()));
        }
        let amount = |text: &str| {
            text.parse::<u64>()
                .map_err(|_| SwapError::InvalidResponse("invalid raw output amount".into()))
        };

        // Parse output amounts as exact integers before checking the slippage floor.
        let out = amount(&self.out_amount)?;
        let min = amount(&self.other_amount_threshold)?;
        if out == 0 || min == 0 {
            return Err(SwapError::InsufficientLiquidity(
                "zero output/minimum".into(),
            ));
        }
        let floor = u128::from(out) * u128::from(10_000 - request.slippage_bps) / 10_000;
        if min > out || u128::from(min) < floor {
            return Err(SwapError::InvalidResponse(
                "minimum output violates requested slippage".into(),
            ));
        }

        // Normalize the provider impact ratio into a finite percentage.
        let impact = self
            .price_impact_pct
            .parse::<f64>()
            .ok()
            .filter(|v| v.is_finite())
            .ok_or_else(|| SwapError::InvalidResponse("missing/invalid price impact".into()))?
            * 100.0;
        if !impact.is_finite() {
            return Err(SwapError::InvalidResponse("invalid price impact".into()));
        }
        Ok(SwapQuote {
            input_mint: self.input_mint.clone(),
            output_mint: self.output_mint.clone(),
            raw_amount: self.in_amount.clone(),
            expected_out_lamports: out,
            min_out_lamports: min,
            expected_out_sol: String::new(),
            min_out_sol: String::new(),
            price_impact_pct: impact,
            slippage_bps: self.slippage_bps,
            route_exists: true,
            route_labels: self
                .route_plan
                .iter()
                .filter_map(|r| r.swap_info.label.clone())
                .collect(),
        }
        .amounts())
    }
}

/// Map a Jupiter failure response to an application error.
/// Only recognized no-route responses on accepted client-error statuses can authorize
/// a burn plan; authentication, rate limits and general API failures remain separate.
pub fn api_error(status: u16, value: &serde_json::Value) -> SwapError {
    let code = value["errorCode"].as_str().unwrap_or("");
    let message = value["error"]
        .as_str()
        .or(value["errorMessage"].as_str())
        .unwrap_or("unexpected API response");
    let detail = format!("HTTP {status}: {code} {message}");
    let upper = detail.to_ascii_uppercase();
    if upper.contains("INSUFFICIENT_LIQUIDITY")
        || upper.contains("INSUFFICIENT LIQUIDITY")
        || upper.contains("COULD_NOT_FIND_ANY_ROUTE")
        || upper.contains("ROUTE_PLAN_DOES_NOT_CONSUME_ALL_THE_AMOUNT")
    {
        SwapError::InsufficientLiquidity(detail)
    } else if matches!(status, 400 | 422)
        && (upper.contains("NO ROUTES FOUND")
            || upper.contains("NO_ROUTES")
            || upper.contains("TOKEN_NOT_TRADABLE")
            || upper.contains("NOT TRADABLE")
            || upper.contains("NOT_TRADABLE"))
    {
        SwapError::NoRoute(detail)
    } else if upper.contains("INSUFFICIENT FUNDS") || upper.contains("INSUFFICIENT_FUNDS") {
        SwapError::InsufficientFunds(detail)
    } else if upper.contains("EXPIRED") || upper.contains("STALE") {
        SwapError::Expired
    } else {
        SwapError::Api(detail)
    }
}

impl SwapProvider for Jupiter {
    async fn build_swap(&self, request: &SwapRequest) -> Result<PreparedSwap> {
        request.validate()?;
        let requested_at = Instant::now();
        let response = self
            .get("/swap/v2/build")
            .query(&[
                ("inputMint", request.mint.to_string()),
                ("outputMint", WRAPPED_SOL.into()),
                ("amount", request.raw_amount.to_string()),
                ("taker", request.wallet.to_string()),
                ("slippageBps", request.slippage_bps.to_string()),
                ("wrapAndUnwrapSol", "true".into()),
                ("nativeDestinationAccount", request.wallet.to_string()),
            ])
            .send()
            .await
            .map_err(|e| SwapError::Api(e.without_url().to_string()))?;

        // Decode provider failure details before attempting successful-response conversion.
        let status = response.status();
        let value: serde_json::Value = response
            .json()
            .await
            .map_err(|e| SwapError::Api(e.without_url().to_string()))?;
        if !status.is_success() || value.get("error").is_some() || value.get("errorCode").is_some()
        {
            return Err(api_error(status.as_u16(), &value));
        }
        let build: BuildResponse =
            serde_json::from_value(value).map_err(|e| SwapError::InvalidResponse(e.to_string()))?;
        let quote = build.quote(request)?;
        Ok(PreparedSwap {
            request: request.clone(),
            quote,
            build: build.try_into()?,
            requested_at,
        })
    }
}

// Map the provider's strings/base64/lookup DTOs exactly once at this boundary.
use crate::core::swap::SwapTransaction;
use base64::{Engine, engine::general_purpose::STANDARD};
use solana_instruction::{AccountMeta, Instruction};
use solana_message::AddressLookupTableAccount;
use solana_pubkey::Pubkey;
use std::str::FromStr;
fn invalid(reason: impl ToString) -> SwapError {
    SwapError::InvalidResponse(reason.to_string())
}
fn address(text: &str) -> Result<Pubkey> {
    Pubkey::from_str(text).map_err(invalid)
}
fn instruction(api: &ApiInstruction) -> Result<Instruction> {
    Ok(Instruction {
        program_id: address(&api.program_id)?,
        accounts: api
            .accounts
            .iter()
            .map(|account| {
                Ok(AccountMeta {
                    pubkey: address(&account.pubkey)?,
                    is_signer: account.is_signer,
                    is_writable: account.is_writable,
                })
            })
            .collect::<Result<_>>()?,
        data: STANDARD.decode(&api.data).map_err(invalid)?,
    })
}

impl TryFrom<BuildResponse> for SwapTransaction {
    type Error = SwapError;
    fn try_from(build: BuildResponse) -> Result<Self> {
        let instructions =
            |items: &[ApiInstruction]| items.iter().map(instruction).collect::<Result<Vec<_>>>();
        let mut lookup_tables = Vec::new();
        for (key, addresses) in build.addresses_by_lookup_table_address.unwrap_or_default() {
            lookup_tables.push(AddressLookupTableAccount {
                key: address(&key)?,
                addresses: addresses
                    .iter()
                    .map(|key| address(key))
                    .collect::<Result<_>>()?,
            });
        }
        Ok(Self {
            compute_budget_instructions: instructions(&build.compute_budget_instructions)?,
            setup_instructions: instructions(&build.setup_instructions)?,
            swap_instruction: instruction(&build.swap_instruction)?,
            cleanup_instruction: build
                .cleanup_instruction
                .as_ref()
                .map(instruction)
                .transpose()?,
            other_instructions: instructions(&build.other_instructions)?,
            tip_instruction: build
                .tip_instruction
                .as_ref()
                .map(instruction)
                .transpose()?,
            lookup_tables,
            blockhash: solana_hash::Hash::new_from_array(build.blockhash_with_metadata.blockhash),
            last_valid_block_height: build.blockhash_with_metadata.last_valid_block_height,
        })
    }
}
