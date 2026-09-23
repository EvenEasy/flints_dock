
use anyhow::Result;
use mpl_token_metadata::{
    accounts::{Edition, MasterEdition, Metadata},
    types::TokenStandard,
    ID as METADATA_PROGRAM_ID,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_commitment_config::CommitmentConfig;
use solana_pubkey::Pubkey as RpcPubkey;
use solana_pubkey_v2::Pubkey as MetaplexPubkey;

use crate::scanner::tokens::TokenAccount;

#[derive(Debug)]
pub struct Nft {
    pub mint: String,
    pub token_account: String,
    pub name: String,
    pub symbol: String,
    pub uri: String,
    pub programmable: bool,
}

pub async fn get_nfts(
    rpc: &RpcClient,
    token_accounts: &[TokenAccount],
) -> Result<Vec<Nft>> {
    let mut nfts = Vec::new();

    let metadata_program = RpcPubkey::new_from_array(
        METADATA_PROGRAM_ID.to_bytes(),
    );

    let mut candidates = 0;
    let mut missing_metadata = 0;
    let mut missing_edition = 0;
    let mut other_standard = 0;

    println!("Token accounts: {}", token_accounts.len());

    for token in token_accounts {
        let raw_amount = token.amount.parse::<u64>().unwrap();

        if raw_amount == 0 || token.decimals != 0 {
            continue;
        }

        candidates += 1;

        let mint: RpcPubkey = token.mint.parse()?;

        let mpl_mint = MetaplexPubkey::new_from_array(
            mint.to_bytes(),
        );

        let (metadata_pda, _) = Metadata::find_pda(&mpl_mint);

        let rpc_pda = RpcPubkey::new_from_array(
            metadata_pda.to_bytes(),
        );

        let Some(account) = rpc
            .get_account_with_commitment(
                &rpc_pda,
                CommitmentConfig::confirmed(),
            )
            .await?
            .value
        else {
            missing_metadata += 1;
            continue;
        };

        if account.owner != metadata_program {
            missing_metadata += 1;
            continue;
        }

        let metadata = match Metadata::from_bytes(&account.data) {
            Ok(metadata) => metadata,
            Err(err) => {
                eprintln!(
                    "Cannot decode metadata for {}: {}",
                    token.mint, err
                );
                continue;
            }
        };

        if metadata.mint != mpl_mint {
            eprintln!("Metadata mint mismatch: {}", token.mint);
            continue;
        }

        let programmable = match metadata.token_standard {
            Some(TokenStandard::NonFungible)
            | Some(TokenStandard::NonFungibleEdition) => false,

            Some(TokenStandard::ProgrammableNonFungible)
            | Some(TokenStandard::ProgrammableNonFungibleEdition) => true,

            None => {
                // Старі NFT можуть не мати token_standard.
                // Перевіряємо наявність справжнього Edition PDA.
                let (edition_pda, _) =
                    MasterEdition::find_pda(&mpl_mint);

                let rpc_edition = RpcPubkey::new_from_array(
                    edition_pda.to_bytes(),
                );

                let edition_account = rpc
                    .get_account_with_commitment(
                        &rpc_edition,
                        CommitmentConfig::confirmed(),
                    )
                    .await?
                    .value;

                let Some(edition) = edition_account else {
                    missing_edition += 1;
                    continue;
                };

                if edition.owner != metadata_program {
                    missing_edition += 1;
                    continue;
                }

                if MasterEdition::from_bytes(&edition.data).is_err()
                    && Edition::from_bytes(&edition.data).is_err()
                {
                    missing_edition += 1;
                    continue;
                }

                false
            }

            _ => {
                other_standard += 1;
                continue;
            }
        };

        nfts.push(Nft {
            mint: token.mint.clone(),
            token_account: token.address.clone(),
            name: metadata.name.trim_end_matches('\0').to_owned(),
            symbol: metadata.symbol.trim_end_matches('\0').to_owned(),
            uri: metadata.uri.trim_end_matches('\0').to_owned(),
            programmable,
        });
    }

    println!("NFT scan:");
    println!("  Candidates: {}", candidates);
    println!("  Missing metadata: {}", missing_metadata);
    println!("  Missing legacy edition: {}", missing_edition);
    println!("  Other token standards: {}", other_standard);
    println!("  Found NFTs: {}", nfts.len());

    Ok(nfts)
}