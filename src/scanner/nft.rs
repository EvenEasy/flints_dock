use crate::{models::*, rpc};
use anyhow::{Result, ensure};
use mpl_token_metadata::{
    accounts::{Edition, MasterEdition, Metadata},
    types::{Key, TokenStandard},
};
use solana_account::Account;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_pubkey::Pubkey;
use std::collections::BTreeMap;

pub fn decode_metadata(account: &Account, mint: &Pubkey) -> Result<Metadata> {
    ensure!(
        account.owner == rpc::to_rpc(mpl_token_metadata::ID),
        "Metadata PDA has wrong program owner"
    );
    let metadata = Metadata::from_bytes(&account.data)?;
    ensure!(
        metadata.key == Key::MetadataV1,
        "Wrong metadata discriminator"
    );
    ensure!(
        metadata.mint.to_bytes() == mint.to_bytes(),
        "Metadata mint mismatch"
    );
    Ok(metadata)
}

pub fn edition_evidence(account: &Account) -> Option<bool> {
    if account.owner != rpc::to_rpc(mpl_token_metadata::ID) {
        return None;
    }
    // Generated Borsh decoders don't validate discriminators: check them explicitly.
    if let Ok(edition) = Edition::from_bytes(&account.data)
        && edition.key == Key::EditionV1
    {
        return Some(true);
    }
    if let Ok(master) = MasterEdition::from_bytes(&account.data)
        && matches!(master.key, Key::MasterEditionV1 | Key::MasterEditionV2)
        // V1 appends two printing mint keys after the common prefix. Its
        // optional max_supply changes that prefix length by eight bytes.
        && (master.key != Key::MasterEditionV1
            || account.data.len() >= 74 + usize::from(master.max_supply.is_some()) * 8)
    {
        return Some(false);
    }
    None
}

pub fn standard_kind(standard: &TokenStandard) -> AssetKind {
    match standard {
        TokenStandard::NonFungible | TokenStandard::NonFungibleEdition => AssetKind::NonFungible,
        TokenStandard::ProgrammableNonFungible | TokenStandard::ProgrammableNonFungibleEdition => {
            AssetKind::ProgrammableNonFungible
        }
        TokenStandard::Fungible | TokenStandard::FungibleAsset => AssetKind::Fungible,
    }
}

pub async fn get_metadata(
    rpc: &RpcClient,
    tokens: &[TokenAccount],
    mints: &[MintInfo],
) -> ScanCollection<MetadataRecord> {
    // Fetch metadata for fungible mints too. Names/symbols are labels, never NFT evidence.
    let keys: BTreeMap<Pubkey, Pubkey> = tokens
        .iter()
        .filter_map(|token| token.mint.parse::<Pubkey>().ok())
        .map(|mint| {
            // Metadata PDA seeds are ["metadata", metadata_program, mint]; Metaplex
            // derives them using its own Pubkey version, converted at this boundary.
            let pda = Metadata::find_pda(&rpc::to_metaplex(mint)).0;
            (mint, rpc::to_rpc(pda))
        })
        .collect();
    let fetched = rpc::multiple_accounts(rpc, &keys.values().copied().collect::<Vec<_>>()).await;
    let mut result = ScanCollection::complete(Vec::new());
    let mut decoded = Vec::new();
    for (mint, pda) in keys {
        match fetched.get(&pda) {
            Some(Ok(Some(account))) => match decode_metadata(account, &mint) {
                Ok(metadata) => decoded.push((mint, metadata)),
                Err(error) => result.issue(format!("Metadata {mint}: {error}")),
            },
            Some(Err(error)) => result.issue(format!("Metadata {mint}: {error}")),
            _ => {} // No metadata is a valid result, not proof of NFT or fungibility.
        }
    }
    let candidate = |mint: &Pubkey| {
        mints
            .iter()
            .any(|info| info.mint == mint.to_string() && info.decimals == 0 && info.supply == 1)
            && tokens.iter().any(|token| {
                token.mint == mint.to_string() && token.raw_amount == 1 && token.decimals == Some(0)
            })
    };
    let editions: BTreeMap<_, _> = decoded
        .iter()
        .filter(|(mint, data)| data.token_standard.is_none() && candidate(mint))
        .map(|(mint, _)| {
            (
                *mint,
                rpc::to_rpc(MasterEdition::find_pda(&rpc::to_metaplex(*mint)).0),
            )
        })
        .collect();
    let edition_accounts =
        rpc::multiple_accounts(rpc, &editions.values().copied().collect::<Vec<_>>()).await;
    for (mint, data) in decoded {
        let metadata = TokenMetadata {
            name: Some(data.name.trim_end_matches('\0').to_owned()),
            symbol: Some(data.symbol.trim_end_matches('\0').to_owned()),
            uri: Some(data.uri.trim_end_matches('\0').to_owned()),
            image_uri: None,
            token_standard: data
                .token_standard
                .as_ref()
                .map(|standard| format!("{standard:?}")),
            source: Some("metaplex".into()),
        };
        let mut nft = None;
        if candidate(&mint) {
            let verified = match &data.token_standard {
                Some(standard)
                    if matches!(
                        standard_kind(standard),
                        AssetKind::NonFungible | AssetKind::ProgrammableNonFungible
                    ) =>
                {
                    Some((
                        standard_kind(standard) == AssetKind::ProgrammableNonFungible,
                        matches!(
                            standard,
                            TokenStandard::NonFungibleEdition
                                | TokenStandard::ProgrammableNonFungibleEdition
                        ),
                        "metaplex_token_standard",
                    ))
                }
                None => match editions
                    .get(&mint)
                    .and_then(|pda| edition_accounts.get(pda))
                {
                    Some(Ok(Some(account))) => {
                        let evidence = edition_evidence(account);
                        if evidence.is_none() {
                            result.issue(format!("{mint}: invalid legacy edition account"));
                        }
                        evidence.map(|edition| (false, edition, "legacy_edition_account"))
                    }
                    Some(Err(error)) => {
                        result.issue(format!("Edition {mint}: {error}"));
                        None
                    }
                    _ => None,
                },
                _ => None,
            };
            if let Some((programmable, edition, evidence)) = verified {
                nft = Some(NftAsset {
                    mint: mint.to_string(),
                    token_accounts: tokens
                        .iter()
                        .filter(|token| token.mint == mint.to_string() && token.raw_amount > 0)
                        .map(|token| token.address.clone())
                        .collect(),
                    metadata: metadata.clone(),
                    programmable,
                    edition,
                    evidence: evidence.into(),
                });
            }
        } else if data.token_standard.as_ref().is_some_and(|standard| {
            matches!(
                standard_kind(standard),
                AssetKind::NonFungible | AssetKind::ProgrammableNonFungible
            )
        }) && tokens
            .iter()
            .any(|token| token.mint == mint.to_string() && token.raw_amount > 0)
        {
            result.issue(format!(
                "{mint}: NFT metadata but mint supply/decimals/holding unavailable or inconsistent"
            ));
        }
        result.items.push(MetadataRecord {
            mint: mint.to_string(),
            metadata,
            nft,
        });
    }
    result
}
