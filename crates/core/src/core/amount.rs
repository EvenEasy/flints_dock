use crate::core::*;
use std::collections::BTreeMap;

/// Return an exact decimal representation of `raw` base units using the mint decimals.
/// Handles the full on-chain `u8` decimal range without floating-point rounding.
pub fn exact_amount(raw: u128, decimals: u8) -> String {
    // String placement avoids both floating-point rounding and 10^decimals
    // overflow (decimals is an on-chain u8, not constrained to the usual 6/9).
    if decimals == 0 {
        return raw.to_string();
    }
    let digits = format!("{:0>width$}", raw, width = usize::from(decimals) + 1);
    let split = digits.len() - usize::from(decimals);
    let fractional = digits[split..].trim_end_matches('0');
    if fractional.is_empty() {
        digits[..split].to_owned()
    } else {
        format!("{}.{}", &digits[..split], fractional)
    }
}

/// Return fungible holdings aggregated by mint and token program, retaining backing accounts.
/// Unknown accounts remain separate; verified NFTs are excluded from this view.
pub fn aggregate_tokens(classified: &[TokenAsset]) -> Vec<TokenAsset> {
    let mut assets: BTreeMap<(String, TokenProgram), TokenAsset> = BTreeMap::new();
    let mut unknown = Vec::new();
    for token in classified {
        if token.kind == AssetKind::Unknown {
            unknown.push(token.clone());
        } else if token.kind.is_fungible() {
            let key = (token.mint.clone(), token.program);
            if let Some(asset) = assets.get_mut(&key) {
                asset.total_raw_amount += token.total_raw_amount;
                asset.accounts.extend(token.accounts.iter().cloned());

                // Conflicting decimals invalidate the display balance, but raw units and backing
                // accounts remain available.
                if asset.decimals != token.decimals {
                    asset.decimals = None;
                }
                asset.balance = asset
                    .decimals
                    .map(|decimals| exact_amount(asset.total_raw_amount, decimals));
            } else {
                assets.insert(key, token.clone());
            }
        }
    }
    assets.into_values().chain(unknown).collect()
}

/// Aggregate every asset kind by mint/program for evidence categories; CLI token filtering stays separate.
/// Any missing valuation or conflicting kind/decimals makes that aggregate unvalued or unknown.
pub fn aggregate_holdings(classified: &[TokenAsset]) -> Vec<TokenAsset> {
    let mut holdings: BTreeMap<(String, TokenProgram), TokenAsset> = BTreeMap::new();
    for token in classified {
        let key = (token.mint.clone(), token.program);
        if let Some(holding) = holdings.get_mut(&key) {
            let prior_amount = holding.total_raw_amount;
            holding.total_raw_amount += token.total_raw_amount;
            holding.accounts.extend(token.accounts.iter().cloned());
            holding.accounts.sort();
            holding.accounts.dedup();
            if holding.kind != token.kind {
                holding.kind = AssetKind::Unknown;
            }
            if holding.decimals != token.decimals {
                holding.decimals = None;
            }
            holding.balance = holding
                .decimals
                .map(|d| exact_amount(holding.total_raw_amount, d));
            if prior_amount == 0 {
                holding.value_usd = token.value_usd;
            } else if token.total_raw_amount > 0 {
                holding.value_usd = holding
                    .value_usd
                    .zip(token.value_usd)
                    .map(|(a, b)| a + b)
                    .filter(|v| v.is_finite());
            }
        } else {
            holdings.insert(key, token.clone());
        }
    }
    holdings.into_values().collect()
}

/// Return account counts and actual lamports potentially recoverable through closure.
/// Recovery is conditional on the stored eligibility assessment, not a fixed rent estimate.
pub fn summarize_accounts(accounts: &[TokenAccount]) -> AccountSummary {
    AccountSummary {
        token_accounts: accounts.len(),
        empty_token_accounts: accounts.iter().filter(|account| account.is_empty()).count(),
        token_account_lamports: accounts
            .iter()
            .map(|account| u128::from(account.lamports))
            .sum(),

        // Actual lamports, not a fixed rent constant. This is a conditional
        // estimate, not a guarantee that a close instruction would succeed.
        potentially_reclaimable_lamports: accounts
            .iter()
            .filter(|account| matches!(account.closure, ClosureAssessment::PotentiallyReclaimable))
            .map(|account| u128::from(account.lamports))
            .sum(),
        closure_review_accounts: accounts
            .iter()
            .filter(|account| matches!(account.closure, ClosureAssessment::NeedsReview(_)))
            .count(),
    }
}

/// Return an approximate USD value for raw units, or `None` for invalid or overflowing values.
/// Canonical balances remain integers; `price` is the USD value of one whole token.
pub fn approximate_value(raw: u128, decimals: u8, price: f64) -> Option<f64> {
    let amount: f64 = exact_amount(raw, decimals).parse().ok()?;
    let value = amount * price;
    (price.is_finite() && price >= 0.0 && value.is_finite()).then_some(value)
}

/// Return the sum of finite known valuations.
/// Returns `None` when no usable values exist or the sum overflows.
pub fn sum_known_values(values: impl IntoIterator<Item = Option<f64>>) -> Option<f64> {
    let mut sum = None;
    for value in values
        .into_iter()
        .flatten()
        .filter(|value| value.is_finite())
    {
        let next = sum.unwrap_or(0.0) + value;
        if !next.is_finite() {
            return None;
        }
        sum = Some(next);
    }
    sum
}
