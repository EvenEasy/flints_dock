pub mod console;
pub mod json;
use crate::models::*;

#[derive(Debug, Clone, Copy, Default)]
pub struct OutputOptions {
    pub show_mint: bool,
    pub show_price: bool,
    pub include_empty: bool,
    pub details: bool,
}

pub fn visible_tokens<'a>(
    portfolio: &'a Portfolio,
    options: &OutputOptions,
) -> Vec<&'a TokenAsset> {
    visible_token_assets(&portfolio.tokens, options)
}

pub fn visible_token_assets<'a>(
    assets: &'a [TokenAsset],
    options: &OutputOptions,
) -> Vec<&'a TokenAsset> {
    let mut tokens: Vec<_> = assets
        .iter()
        .filter(|token| options.include_empty || token.total_raw_amount > 0)
        .collect();
    tokens.sort_by(|a, b| {
        let a_value = a.value_usd.filter(|value| value.is_finite());
        let b_value = b.value_usd.filter(|value| value.is_finite());
        match (a_value, b_value) {
            (Some(a), Some(b)) => b.total_cmp(&a),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            _ => std::cmp::Ordering::Equal,
        }
        .then_with(|| token_name(a).cmp(token_name(b)))
        .then_with(|| a.mint.cmp(&b.mint))
    });
    tokens
}

pub fn token_name(token: &TokenAsset) -> &str {
    token
        .metadata
        .symbol
        .as_deref()
        .filter(|text| !text.trim().is_empty())
        .or(token
            .metadata
            .name
            .as_deref()
            .filter(|text| !text.trim().is_empty()))
        .unwrap_or("Unknown token")
}

pub struct NftRow<'a> {
    pub name: &'a str,
    pub kind: &'static str,
    pub asset_id: &'a str,
    pub collection: Option<&'a CollectionInfo>,
    pub uri: Option<&'a str>,
    pub accounts: &'a [String],
    pub lamports: u128,
}

pub fn nft_rows(portfolio: &Portfolio) -> Vec<NftRow<'_>> {
    let mut rows: Vec<_> = portfolio
        .classic_nfts
        .iter()
        .map(|nft| NftRow {
            name: nft.metadata.name.as_deref().unwrap_or("Unnamed NFT"),
            kind: match (nft.programmable, nft.edition) {
                (true, true) => "Programmable edition",
                (true, false) => "Programmable",
                (false, true) => "Classic edition",
                (false, false) => "Classic",
            },
            asset_id: &nft.mint,
            collection: nft.metadata.collection.as_ref(),
            uri: nft.metadata.uri.as_deref(),
            accounts: &nft.token_accounts,
            lamports: portfolio
                .token_accounts
                .iter()
                .filter(|account| nft.token_accounts.contains(&account.address))
                .map(|account| u128::from(account.lamports))
                .sum(),
        })
        .collect();
    rows.extend(portfolio.core_assets.iter().map(|asset| NftRow {
        name: &asset.name,
        kind: "Core",
        asset_id: &asset.address,
        collection: asset.collection.as_ref(),
        uri: Some(&asset.uri),
        accounts: &[],
        lamports: asset.lamports.into(),
    }));
    rows.sort_by(|a, b| a.name.cmp(b.name).then_with(|| a.asset_id.cmp(b.asset_id)));
    rows
}
