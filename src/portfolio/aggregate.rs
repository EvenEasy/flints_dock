use crate::models::*;
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

pub fn aggregate_tokens(
    accounts: &[TokenAccount],
    mints: &[MintInfo],
    records: &[MetadataRecord],
) -> Vec<TokenAsset> {
    let mut assets: BTreeMap<(String, TokenProgram), TokenAsset> = BTreeMap::new();
    for account in accounts {
        let asset = assets
            .entry((account.mint.clone(), account.program))
            .or_insert_with(|| {
                let mint = mints
                    .iter()
                    .find(|mint| mint.mint == account.mint && mint.program == account.program);
                let record = records.iter().find(|record| record.mint == account.mint);
                let mut metadata = record
                    .map(|record| record.metadata.clone())
                    .or_else(|| mint.map(|mint| mint.metadata.clone()))
                    .unwrap_or_default();
                if metadata.image_uri.is_none() {
                    metadata.image_uri = mint.and_then(|mint| mint.metadata.image_uri.clone());
                }
                let kind = if let Some(nft) = record.and_then(|record| record.nft.as_ref()) {
                    if nft.programmable {
                        AssetKind::ProgrammableNonFungible
                    } else {
                        AssetKind::NonFungible
                    }
                } else if metadata
                    .token_standard
                    .as_deref()
                    .is_some_and(|standard| matches!(standard, "Fungible" | "FungibleAsset"))
                    || (metadata.token_standard.is_none()
                        && mint.is_some_and(|mint| mint.decimals > 0))
                {
                    AssetKind::Fungible
                } else {
                    AssetKind::Unknown
                };
                TokenAsset {
                    mint: account.mint.clone(),
                    program: account.program,
                    total_raw_amount: 0,
                    decimals: account.decimals,
                    balance: None,
                    accounts: Vec::new(),
                    kind,
                    metadata,
                    price: None,
                    value_usd: None,
                }
            });
        // Each amount is u64; u128 comfortably holds a wallet's account sum.
        asset.total_raw_amount += u128::from(account.raw_amount);
        if asset.decimals != account.decimals {
            asset.decimals = None;
        }
        asset.accounts.push(account.address.clone());
    }
    assets
        .into_values()
        .map(|mut asset| {
            asset.balance = asset
                .decimals
                .map(|decimals| exact_amount(asset.total_raw_amount, decimals));
            asset
        })
        .collect()
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
