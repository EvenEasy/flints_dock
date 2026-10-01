use crate::core::*;
use std::collections::BTreeMap;

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

/// Combine only positively classified fungibles. Unknown accounts remain separate;
/// NFT accounts belong to the NFT category and the unaggregated all-token view.
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

pub fn approximate_value(raw: u128, decimals: u8, price: f64) -> Option<f64> {
    let amount: f64 = exact_amount(raw, decimals).parse().ok()?;
    let value = amount * price;
    (price.is_finite() && price >= 0.0 && value.is_finite()).then_some(value)
}

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
