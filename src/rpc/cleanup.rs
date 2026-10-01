use crate::{
    classification::classify_token_accounts,
    cleanup::{plan::unsupported_reason, *},
    models::TokenProgram,
    scanner,
    swap::{PreparedSwap, Result, SwapError, SwapLimits},
};
use serde_json::{Value, json};
use solana_account_decoder::{UiAccountEncoding, encode_ui_account};
use solana_client::{
    nonblocking::rpc_client::RpcClient, rpc_request::RpcRequest, rpc_response::RpcKeyedAccount,
};
use solana_commitment_config::CommitmentConfig;
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use spl_token_2022_interface::instruction::{burn_checked, close_account, transfer_checked};

const ASSOCIATED_TOKEN: &str = "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";
fn invalid(reason: impl ToString) -> SwapError {
    SwapError::InvalidRequest(reason.to_string())
}
fn rpc_error(reason: impl ToString) -> SwapError {
    SwapError::Rpc(reason.to_string())
}
fn pubkey(value: &str) -> Result<Pubkey> {
    value.parse().map_err(invalid)
}

pub fn associated_address(owner: &Pubkey, mint: &Pubkey, program: TokenProgram) -> Pubkey {
    Pubkey::find_program_address(
        &[owner.as_ref(), program.id().as_ref(), mint.as_ref()],
        &ASSOCIATED_TOKEN.parse().expect("ATA program"),
    )
    .0
}

pub fn burn_instruction(asset: &CleanupAsset, owner: &Pubkey) -> Result<Instruction> {
    if let Some(reason) = unsupported_reason(asset, &owner.to_string()) {
        return Err(invalid(reason));
    }
    if asset.account.raw_amount == 0 || asset.account.is_native {
        return Err(invalid("burn requires a nonempty non-native account"));
    }
    burn_checked(
        &asset.account.program.id(),
        &pubkey(&asset.account.address)?,
        &pubkey(&asset.account.mint)?,
        owner,
        &[],
        asset.account.raw_amount,
        asset
            .account
            .decimals
            .ok_or_else(|| invalid("missing decimals"))?,
    )
    .map_err(invalid)
}
pub fn close_instruction(asset: &CleanupAsset, owner: &Pubkey) -> Result<Instruction> {
    if let Some(reason) = unsupported_reason(asset, &owner.to_string()) {
        return Err(invalid(reason));
    }
    if asset.account.raw_amount != 0 {
        return Err(invalid("refusing to close a nonempty account"));
    }
    close_account(
        &asset.account.program.id(),
        &pubkey(&asset.account.address)?,
        owner,
        owner,
        &[],
    )
    .map_err(invalid)
}

/// Move exactly this auxiliary account's balance into the default Jupiter input ATA.
/// These instructions and the swap share one atomic transaction, including removal
/// of a newly created staging ATA. Existing ATA holdings are not added to inAmount.
pub fn source_instructions(
    asset: &CleanupAsset,
    owner: &Pubkey,
    ata_exists: bool,
) -> Result<(Vec<Instruction>, Vec<Instruction>)> {
    let mint = pubkey(&asset.account.mint)?;
    let source = pubkey(&asset.account.address)?;
    let ata = associated_address(owner, &mint, asset.account.program);
    if source == ata {
        return Ok((vec![], vec![]));
    }
    let mut prefix = vec![];
    let mut suffix = vec![];
    if !ata_exists {
        // Standard ATA CreateIdempotent ABI: discriminator 1 and six accounts.
        // https://github.com/solana-program/associated-token-account/blob/main/interface/src/instruction.rs
        prefix.push(Instruction {
            program_id: ASSOCIATED_TOKEN.parse().expect("ATA program"),
            data: vec![1],
            accounts: vec![
                AccountMeta::new(*owner, true),
                AccountMeta::new(ata, false),
                AccountMeta::new_readonly(*owner, false),
                AccountMeta::new_readonly(mint, false),
                AccountMeta::new_readonly(Pubkey::default(), false),
                AccountMeta::new_readonly(asset.account.program.id(), false),
            ],
        });
        suffix.push(
            close_account(&asset.account.program.id(), &ata, owner, owner, &[]).map_err(invalid)?,
        );
    }
    prefix.push(
        transfer_checked(
            &asset.account.program.id(),
            &source,
            &mint,
            &ata,
            owner,
            &[],
            asset.account.raw_amount,
            asset
                .account
                .decimals
                .ok_or_else(|| invalid("missing decimals"))?,
        )
        .map_err(invalid)?,
    );
    Ok((prefix, suffix))
}

/// Extract observed wallet movement and actual closed-account lamports from confirmed
/// metadata. Missing metadata remains unknown; quote estimates never become receipts.
pub fn transaction_accounting(
    value: &Value,
    owner: &str,
    closed_account: Option<&str>,
) -> (Option<i128>, Option<u64>) {
    let meta = &value["meta"];
    if !meta.is_object() || !meta["err"].is_null() {
        return (None, None);
    }
    let Some(keys) = value["transaction"]["message"]["accountKeys"].as_array() else {
        return (None, None);
    };
    let mut keys: Vec<_> = keys.iter().filter_map(Value::as_str).collect();
    for group in ["writable", "readonly"] {
        keys.extend(
            meta["loadedAddresses"][group]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str),
        );
    }
    let balances = |address: &str| -> Option<(u64, u64)> {
        let index = keys.iter().position(|key| *key == address)?;
        Some((
            meta["preBalances"][index].as_u64()?,
            meta["postBalances"][index].as_u64()?,
        ))
    };
    let delta = balances(owner).map(|(before, after)| i128::from(after) - i128::from(before));
    let reclaimed = closed_account
        .and_then(balances)
        .and_then(|(before, after)| (after == 0).then_some(before));
    (delta, reclaimed)
}
async fn receipt(
    rpc: &RpcClient,
    operation: CleanupOperation,
    signature: String,
    owner: &Pubkey,
    source: &str,
) -> OperationReceipt {
    let value: std::result::Result<Option<Value>, _> = rpc.send(RpcRequest::GetTransaction,
        json!([signature, {"encoding":"json","commitment":"confirmed","maxSupportedTransactionVersion":0}])).await;
    let (wallet_delta_lamports, reclaimed_lamports) = value
        .ok()
        .flatten()
        .map(|value| {
            transaction_accounting(
                &value,
                &owner.to_string(),
                (operation == CleanupOperation::Close).then_some(source),
            )
        })
        .unwrap_or((None, None));
    OperationReceipt {
        operation,
        signature,
        wallet_delta_lamports,
        reclaimed_lamports,
    }
}

impl CleanupExecutor for RpcClient {
    async fn refresh(&self, address: &str, owner: &Pubkey) -> Result<Option<CleanupAsset>> {
        let key = pubkey(address)?;
        let Some(raw) = self
            .get_account_with_commitment(&key, CommitmentConfig::confirmed())
            .await
            .map_err(rpc_error)?
            .value
        else {
            return Ok(None);
        };
        let program = if raw.owner == TokenProgram::Legacy.id() {
            TokenProgram::Legacy
        } else if raw.owner == TokenProgram::Token2022.id() {
            TokenProgram::Token2022
        } else {
            return Err(invalid("source is no longer a token account"));
        };
        let keyed = RpcKeyedAccount {
            pubkey: address.into(),
            account: encode_ui_account(&key, &raw, UiAccountEncoding::Base64, None, None),
        };
        let mut accounts =
            vec![scanner::tokens::parse_account(&keyed, program, owner).map_err(rpc_error)?];
        let mints = scanner::metadata::get_mints(self, &mut accounts).await;
        let metadata = scanner::nft::get_metadata(self, &accounts, &mints.items).await;
        if !mints.status.is_complete() || !metadata.status.is_complete() {
            return Err(rpc_error("cannot fully revalidate mint/metadata"));
        }
        let classified = classify_token_accounts(&accounts, &mints.items, &metadata.items);
        Ok(Some(CleanupAsset {
            account: accounts.remove(0),
            mint: mints.items.into_iter().next(),
            kind: classified
                .first()
                .map(|asset| asset.kind)
                .unwrap_or(crate::models::AssetKind::Unknown),
        }))
    }

    async fn perform(
        &self,
        operation: CleanupOperation,
        asset: &CleanupAsset,
        fresh: Option<PreparedSwap>,
        signer: &Keypair,
        limits: &SwapLimits,
    ) -> Result<OperationReceipt> {
        let owner = signer.pubkey();
        let current = self
            .refresh(&asset.account.address, &owner)
            .await?
            .ok_or_else(|| invalid("account disappeared"))?;
        if current.account.mint != asset.account.mint
            || current.account.program != asset.account.program
            || current.account.decimals != asset.account.decimals
            || current.account.raw_amount != asset.account.raw_amount
        {
            return Err(invalid("source changed before submission"));
        }
        if let Some(reason) = unsupported_reason(&current, &owner.to_string()) {
            return Err(invalid(reason));
        }
        let signature = match operation {
            CleanupOperation::Burn => {
                super::transactions::send_instructions(
                    self,
                    signer,
                    &[burn_instruction(&current, &owner)?],
                    limits,
                )
                .await?
            }
            CleanupOperation::Close => {
                super::transactions::send_instructions(
                    self,
                    signer,
                    &[close_instruction(&current, &owner)?],
                    limits,
                )
                .await?
            }
            CleanupOperation::Swap => {
                let fresh = fresh.ok_or_else(|| invalid("missing fresh swap"))?;
                if fresh.request.wallet != owner
                    || fresh.request.mint.to_string() != current.account.mint
                    || fresh.request.raw_amount != current.account.raw_amount
                {
                    return Err(invalid("swap does not match source"));
                }
                let ata = associated_address(&owner, &fresh.request.mint, current.account.program);
                let ata_state = if ata.to_string() == current.account.address {
                    Some(current.clone())
                } else {
                    self.refresh(&ata.to_string(), &owner).await?
                };
                if let Some(ata_state) = &ata_state {
                    if let Some(reason) = unsupported_reason(ata_state, &owner.to_string()) {
                        return Err(invalid(format!("input ATA: {reason}")));
                    }
                    if ata_state.account.state != "Initialized"
                        || ata_state.account.mint != current.account.mint
                        || ata_state.account.program != current.account.program
                    {
                        return Err(invalid("input ATA is incompatible"));
                    }
                }
                let (prefix, suffix) = source_instructions(&current, &owner, ata_state.is_some())?;
                super::swap::submit_with_instructions(self, fresh, signer, limits, &prefix, &suffix)
                    .await?
            }
        };
        Ok(receipt(self, operation, signature, &owner, &asset.account.address).await)
    }
}
