use crate::{core::amount::exact_amount, core::*};

/// Inputs come from validated mint and Metaplex PDA decoders. A label, URI or
/// Token-2022 metadata extension alone is never evidence of NFT semantics.
pub fn classify(
    account: &TokenAccount,
    mint: Option<&MintInfo>,
    record: Option<&MetadataRecord>,
) -> AssetKind {
    let Some(mint) =
        mint.filter(|mint| mint.mint == account.mint && mint.program == account.program)
    else {
        return AssetKind::Unknown;
    };
    if account.decimals != Some(mint.decimals) {
        return AssetKind::Unknown;
    }
    let record = record.filter(|record| record.mint == account.mint);
    let standard = record.and_then(|record| record.metadata.token_standard.as_deref());
    let nft_semantics = mint.decimals == 0 && mint.supply <= 1 && account.raw_amount <= mint.supply;
    match standard {
        Some("Fungible") if mint.decimals > 0 => AssetKind::Fungible,
        Some("FungibleAsset") if mint.decimals == 0 => AssetKind::FungibleAsset,
        Some("NonFungible") if nft_semantics => AssetKind::NonFungible,
        Some("NonFungibleEdition") if nft_semantics => AssetKind::NonFungibleEdition,
        Some("ProgrammableNonFungible") if nft_semantics => AssetKind::ProgrammableNonFungible,
        Some("ProgrammableNonFungibleEdition") if nft_semantics => {
            AssetKind::ProgrammableNonFungibleEdition
        }
        Some(_) => AssetKind::Unknown, // Conflicting metadata is not safe to price.
        None => {
            if nft_semantics && let Some(nft) = record.and_then(|record| record.nft.as_ref()) {
                match (nft.programmable, nft.edition) {
                    (false, false) => AssetKind::NonFungible,
                    (false, true) => AssetKind::NonFungibleEdition,
                    (true, false) => AssetKind::ProgrammableNonFungible,
                    (true, true) => AssetKind::ProgrammableNonFungibleEdition,
                }
            } else if mint.decimals > 0 {
                AssetKind::Fungible
            } else {
                AssetKind::Unknown
            }
        }
    }
}

/// Preserve one row per token account, including unknown and empty accounts.
/// Aggregation and category selection happen after this shared classification.
pub fn classify_token_accounts(
    accounts: &[TokenAccount],
    mints: &[MintInfo],
    records: &[MetadataRecord],
) -> Vec<TokenAsset> {
    let mints: std::collections::BTreeMap<_, _> = mints
        .iter()
        .map(|mint| ((mint.mint.as_str(), mint.program), mint))
        .collect();
    let records: std::collections::BTreeMap<_, _> = records
        .iter()
        .map(|record| (record.mint.as_str(), record))
        .collect();
    accounts
        .iter()
        .map(|account| {
            let mint = mints
                .get(&(account.mint.as_str(), account.program))
                .copied();
            let record = records.get(account.mint.as_str()).copied();
            let mut metadata = record
                .map(|record| record.metadata.clone())
                .or_else(|| mint.map(|mint| mint.metadata.clone()))
                .unwrap_or_default();
            if metadata.image_uri.is_none() {
                metadata.image_uri = mint.and_then(|mint| mint.metadata.image_uri.clone());
            }
            TokenAsset {
                mint: account.mint.clone(),
                program: account.program,
                total_raw_amount: account.raw_amount.into(),
                decimals: account.decimals,
                balance: account
                    .decimals
                    .map(|decimals| exact_amount(account.raw_amount.into(), decimals)),
                accounts: vec![account.address.clone()],
                kind: classify(account, mint, record),
                metadata,
                price: None,
                value_usd: None,
            }
        })
        .collect()
}
