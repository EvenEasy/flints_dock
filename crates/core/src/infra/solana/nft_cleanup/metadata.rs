//! Token Metadata master, print and programmable NFT burn preparation.
use super::blocked;
use super::{account, invalid, io, key};
use crate::{
    app::{cleanup::*, swap::*},
    core::{nft_cleanup::*, *},
    infra::solana,
};
use serde_json::json;
use solana_account::Account;
use solana_instruction::Instruction;
use solana_pubkey::Pubkey;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
/// Translate the older Metaplex instruction types without changing keys, privileges or bytes.
fn metadata_instruction(ix: mpl_token_metadata::instructions::BurnV1, amount: u64) -> Instruction {
    let ix = ix.instruction(mpl_token_metadata::instructions::BurnV1InstructionArgs { amount });
    Instruction {
        program_id: Pubkey::new_from_array(ix.program_id.to_bytes()),
        data: ix.data,
        accounts: ix
            .accounts
            .into_iter()
            .map(|m| solana_instruction::AccountMeta {
                pubkey: Pubkey::new_from_array(m.pubkey.to_bytes()),
                is_signer: m.is_signer,
                is_writable: m.is_writable,
            })
            .collect(),
    }
}

pub(super) async fn metadata_burn(
    rpc: &RpcClient,
    target: &NftTarget,
    owner: &Pubkey,
    kind: AssetKind,
) -> Result<PreparedNftBurn> {
    use mpl_token_metadata::{
        accounts::{Edition, EditionMarker, MasterEdition, Metadata, TokenRecord},
        types::{Key, TokenState},
    };
    let mint = key(target
        .mint
        .as_deref()
        .ok_or_else(|| invalid("NFT mint missing"))?)?;
    let token = key(target
        .token_account
        .as_deref()
        .ok_or_else(|| invalid("NFT source missing"))?)?;
    let current = rpc
        .refresh(&token.to_string(), owner)
        .await?
        .ok_or_else(|| invalid("NFT source absent"))?;
    // Frozen pNFT accounts are normal: only Token Metadata may consume them. Generic SPL burn stays forbidden.
    if current.kind != kind
        || !kind.is_nft()
        || current.account.owner != owner.to_string()
        || current.account.mint != mint.to_string()
        || current.account.program != TokenProgram::Legacy
        || current.account.raw_amount != 1
        || current.account.decimals != Some(0)
        || current
            .account
            .close_authority
            .as_deref()
            .unwrap_or(&current.account.owner)
            != owner.to_string()
        || current.account.is_native
        || !current.account.extension_types.is_empty()
        || (!kind.is_programmable() && current.account.state != "Initialized")
    {
        return Err(invalid(
            "NFT identity, standard, amount, state or authority is not eligible",
        ));
    }
    let metadata_key = solana::to_rpc(Metadata::find_pda(&solana::to_metaplex(mint)).0);
    let metadata_account =
        account(rpc, &metadata_key, solana::to_rpc(mpl_token_metadata::ID)).await?;
    let metadata = solana::scan::nft::decode_metadata(&metadata_account, &mint).map_err(invalid)?;
    let edition_key = Pubkey::find_program_address(
        &[
            b"metadata",
            mpl_token_metadata::ID.as_ref(),
            mint.as_ref(),
            b"edition",
        ],
        &solana::to_rpc(mpl_token_metadata::ID),
    )
    .0;
    let edition_account =
        account(rpc, &edition_key, solana::to_rpc(mpl_token_metadata::ID)).await?;
    let edition = solana::scan::nft::edition_evidence(&edition_account)
        .ok_or_else(|| invalid("NFT edition evidence unavailable"))?;
    let expected_edition = matches!(
        kind,
        AssetKind::NonFungibleEdition | AssetKind::ProgrammableNonFungibleEdition
    );
    if edition != expected_edition {
        return Err(invalid("NFT edition standard changed"));
    }
    let edition_identity = if edition {
        json!({"data":edition_account.data})
    } else {
        let master = MasterEdition::from_bytes(&edition_account.data).map_err(invalid)?;
        json!({"key":format!("{:?}",master.key),"maxSupply":master.max_supply,"tail":edition_account.data[9..]})
    };
    let edition_count = if edition {
        0
    } else {
        MasterEdition::from_bytes(&edition_account.data)
            .map_err(invalid)?
            .supply
    };
    let mut edition_parent = None;
    let mut master_edition = None;
    let mut master_mint = None;
    let mut master_token = None;
    let mut marker = None;
    if edition {
        let print = Edition::from_bytes(&edition_account.data).map_err(invalid)?;
        // The edition stores its parent PDA, not the parent's mint. Discover that mint by its mint authority.
        let parents = solana::program_accounts(
            rpc,
            &TokenProgram::Legacy.id(),
            vec![
                solana::memcmp(0, 1u32.to_le_bytes().to_vec()),
                solana::memcmp(4, print.parent.to_bytes().to_vec()),
            ],
        )
        .await
        .map_err(io)?;
        let mut candidates = vec![];
        for (address, value) in parents {
            let Some(data) = value.data.decode() else {
                continue;
            };
            if data.len() != 82 {
                continue;
            }
            let raw = Account {
                lamports: value.lamports,
                data,
                owner: TokenProgram::Legacy.id(),
                executable: value.executable,
                rent_epoch: value.rent_epoch,
            };
            let info = solana::scan::metadata::parse_mint(&address, &raw, TokenProgram::Legacy);
            if let Ok(info) = info
                && info.decimals == 0
                && info.supply == 1
            {
                let pda = Pubkey::find_program_address(
                    &[
                        b"metadata",
                        mpl_token_metadata::ID.as_ref(),
                        address.as_ref(),
                        b"edition",
                    ],
                    &solana::to_rpc(mpl_token_metadata::ID),
                )
                .0;
                if pda.to_bytes() == print.parent.to_bytes() {
                    candidates.push(address);
                }
            }
        }
        if candidates.len() != 1 {
            return Err(invalid(
                "Edition parent mint could not be uniquely verified",
            ));
        }
        let parent_mint = candidates[0];
        let parent_account = account(
            rpc,
            &solana::to_rpc(print.parent),
            solana::to_rpc(mpl_token_metadata::ID),
        )
        .await?;
        if solana::scan::nft::edition_evidence(&parent_account) != Some(false) {
            return Err(invalid("Edition parent is not a master edition"));
        }
        let largest = rpc
            .get_token_largest_accounts(&parent_mint)
            .await
            .map_err(io)?;
        let parent_token = largest
            .iter()
            .find(|a| a.amount.amount == "1")
            .ok_or_else(|| invalid("Master edition token evidence unavailable"))?
            .address
            .parse::<Pubkey>()
            .map_err(invalid)?;
        let parent_raw = account(rpc, &parent_token, TokenProgram::Legacy.id()).await?;
        let parent_info = spl_token_2022_interface::extension::StateWithExtensions::<
            spl_token_2022_interface::state::Account,
        >::unpack(&parent_raw.data)
        .map_err(invalid)?;
        if parent_info.base.mint.to_bytes() != parent_mint.to_bytes()
            || parent_info.base.amount != 1
        {
            return Err(invalid("Master edition token account identity changed"));
        }
        let marker_key = Pubkey::find_program_address(
            &[
                b"metadata",
                mpl_token_metadata::ID.as_ref(),
                parent_mint.as_ref(),
                b"edition",
                (print.edition / 248).to_string().as_bytes(),
            ],
            &solana::to_rpc(mpl_token_metadata::ID),
        )
        .0;
        let marker_account =
            account(rpc, &marker_key, solana::to_rpc(mpl_token_metadata::ID)).await?;
        if marker_account.data.first() != Some(&(Key::EditionMarker as u8)) {
            return Err(invalid("Edition marker format unsupported"));
        }
        let marker_data = EditionMarker::from_bytes(&marker_account.data).map_err(invalid)?;
        let bit = (print.edition % 248) as usize;
        if marker_data.ledger[bit / 8] & (1 << (7 - bit % 8)) == 0 {
            return Err(invalid(
                "Edition marker does not contain the selected print",
            ));
        }
        edition_parent = Some(parent_mint.to_string());
        master_edition = Some(print.parent);
        master_mint = Some(solana::to_metaplex(parent_mint));
        master_token = Some(solana::to_metaplex(parent_token));
        marker = Some(solana::to_metaplex(marker_key));
    }
    let token_record = if kind.is_programmable() {
        let address =
            TokenRecord::find_pda(&solana::to_metaplex(mint), &solana::to_metaplex(token)).0;
        let record = account(
            rpc,
            &solana::to_rpc(address),
            solana::to_rpc(mpl_token_metadata::ID),
        )
        .await?;
        let record = TokenRecord::from_bytes(&record.data).map_err(invalid)?;
        if record.key != Key::TokenRecord || record.state != TokenState::Unlocked {
            return Err(blocked(
                CleanupReasonCode::Frozen,
                "Programmable NFT token record is locked or unsupported",
            ));
        }
        Some(address)
    } else {
        None
    };
    let collection_metadata =
        if let Some(collection) = metadata.collection.as_ref().filter(|c| c.verified) {
            let address = Metadata::find_pda(&collection.key).0;
            let raw = account(
                rpc,
                &solana::to_rpc(address),
                solana::to_rpc(mpl_token_metadata::ID),
            )
            .await?;
            solana::scan::nft::decode_metadata(&raw, &solana::to_rpc(collection.key))
                .map_err(invalid)?;
            Some(address)
        } else {
            None
        };
    let instructions = vec![metadata_instruction(
        mpl_token_metadata::instructions::BurnV1 {
            authority: solana::to_metaplex(*owner),
            collection_metadata,
            metadata: solana::to_metaplex(metadata_key),
            edition: Some(solana::to_metaplex(edition_key)),
            mint: solana::to_metaplex(mint),
            token: solana::to_metaplex(token),
            master_edition,
            master_edition_mint: master_mint,
            master_edition_token: master_token,
            edition_marker: marker,
            token_record,
            system_program: solana::to_metaplex(Pubkey::default()),
            sysvar_instructions: "Sysvar1nstructions1111111111111111111111111"
                .parse()
                .expect("sysvar"),
            spl_token_program: solana::to_metaplex(TokenProgram::Legacy.id()),
        },
        1,
    )];
    // Preserve the approved on-chain metadata bytes; changes cannot silently enlarge the burn scope.
    Ok(PreparedNftBurn {
        identity: json!({"target":target,"metadata":metadata_account.data,"edition":edition_identity}),
        instructions,
        edition_parent,
        edition_count,
    })
}
