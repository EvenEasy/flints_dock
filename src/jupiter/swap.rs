use super::{Jupiter, WRAPPED_SOL};
use crate::swap::{PreparedSwap, Result, SwapError, SwapProvider, SwapQuote, SwapRequest};
use serde::Deserialize;
use std::{collections::BTreeMap, time::Instant};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiAccount {
    pub pubkey: String,
    pub is_signer: bool,
    pub is_writable: bool,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiInstruction {
    pub program_id: String,
    pub accounts: Vec<ApiAccount>,
    pub data: String,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockhashMetadata {
    pub blockhash: [u8; 32],
    pub last_valid_block_height: u64,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteStep {
    pub swap_info: RouteInfo,
}
#[derive(Debug, Clone, Deserialize)]
pub struct RouteInfo {
    pub label: Option<String>,
}
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
    pub fn quote(&self, request: &SwapRequest) -> Result<SwapQuote> {
        request.validate()?;
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
            build,
            requested_at,
        })
    }
}
