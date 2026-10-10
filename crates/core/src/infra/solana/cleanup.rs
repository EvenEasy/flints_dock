use crate::{
    app::cleanup::*,
    app::swap::{PreparedSwap, Result, SwapError, SwapLimits},
    core::TokenProgram,
    core::classification::classify_token_accounts,
    infra::solana::scan as scanner,
};
use serde_json::{Value, json};
use solana_account_decoder::{UiAccountEncoding, encode_ui_account};
use solana_commitment_config::CommitmentConfig;
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_rpc_client_api::{request::RpcRequest, response::RpcKeyedAccount};
use solana_signer::Signer;
use spl_token_2022_interface::instruction::{burn_checked, close_account, transfer_checked};

const ASSOCIATED_TOKEN: &str = "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";
fn invalid(reason: impl ToString) -> SwapError {
    SwapError::InvalidRequest(reason.to_string())
}
fn rpc_error(reason: impl ToString) -> SwapError {
    SwapError::Rpc(super::safe_error(reason))
}
fn pubkey(value: &str) -> Result<Pubkey> {
    value.parse().map_err(invalid)
}

/// Return the wallet ATA PDA for a mint and its token program.
/// Seeds include the token program so legacy SPL and Token-2022 addresses remain distinct.
pub fn associated_address(owner: &Pubkey, mint: &Pubkey, program: TokenProgram) -> Pubkey {
    Pubkey::find_program_address(
        &[owner.as_ref(), program.id().as_ref(), mint.as_ref()],
        &ASSOCIATED_TOKEN.parse().expect("ATA program"),
    )
    .0
}

/// Return a full-balance BurnChecked instruction using verified mint decimals.
/// Rejects ineligible, empty or native-backed accounts; `owner` must satisfy cleanup authority
/// checks.
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

/// Return a CloseAccount instruction paying the selected owner wallet.
/// Rejects nonempty or ineligible accounts instead of assuming a prior swap or burn emptied them.
pub fn close_instruction(asset: &CleanupAsset, owner: &Pubkey) -> Result<Instruction> {
    if let Some(reason) = unsupported_reason(asset, &owner.to_string()) {
        return Err(invalid(reason));
    }
    if asset.account.raw_amount != 0 && !asset.account.is_native {
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

/// Return `(prefix, suffix)` instructions for an account-scoped swap through the input ATA.
/// When `ata_exists` is false, create and later close a staging ATA. Existing ATA balances
/// are not included in the swap amount. Both lists must surround the swap in one atomic
/// transaction.
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

/// Return `(wallet_delta, reclaimed_lamports)` from successful confirmed transaction metadata.
/// `closed_account` selects a source whose pre-balance is counted only when its post-balance
/// is zero. Missing metadata stays `None`; wallet delta includes transaction fees and rent effects.
pub fn transaction_accounting(
    value: &Value,
    owner: &str,
    closed_account: Option<&str>,
) -> (Option<i128>, Option<u64>) {
    let meta = &value["meta"];
    if !meta.is_object() {
        return (None, None);
    }
    let Some(keys) = value["transaction"]["message"]["accountKeys"].as_array() else {
        return (None, None);
    };
    let mut keys: Vec<_> = keys.iter().filter_map(Value::as_str).collect();

    // Versioned transaction balances index static keys first, then loaded writable and readonly
    // keys.
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
        .filter(|_| meta["err"].is_null())
        .and_then(balances)
        .and_then(|(before, after)| (after == 0).then_some(before));
    (
        delta,
        if closed_account.is_some() && !meta["err"].is_null() && delta.is_some() {
            Some(0)
        } else {
            reclaimed
        },
    )
}
async fn receipt(
    rpc: &RpcClient,
    operation: CleanupOperation,
    signature: String,
    owner: &Pubkey,
    source: &str,
) -> OperationReceipt {
    // Accounting may lag confirmation; missing metadata must not invalidate a confirmed receipt.
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
        // Empty account closure needs only its decoded token-program state and authority.
        // Avoid making closure depend on unrelated mint/metadata RPC availability.
        if accounts[0].raw_amount == 0 {
            return Ok(Some(CleanupAsset {
                account: accounts.remove(0),
                mint: None,
                kind: crate::core::AssetKind::Unknown,
            }));
        }
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
                .unwrap_or(crate::core::AssetKind::Unknown),
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
        self.perform_observed(operation, asset, fresh, signer, limits, &())
            .await
    }

    async fn failed_receipt(
        &self,
        operation: CleanupOperation,
        signature: &str,
        owner: &Pubkey,
    ) -> Option<OperationReceipt> {
        Some(receipt(self, operation, signature.into(), owner, "").await)
    }

    async fn perform_observed(
        &self,
        operation: CleanupOperation,
        asset: &CleanupAsset,
        fresh: Option<PreparedSwap>,
        signer: &Keypair,
        limits: &SwapLimits,
        observer: &dyn crate::core::progress::CleanupObserver,
    ) -> Result<OperationReceipt> {
        let owner = signer.pubkey();
        let context = crate::core::progress::PendingSubmission {
            wallet: owner.to_string(),
            account: asset.account.address.clone(),
            mint: asset.account.mint.clone(),
            operation,
            signature: String::new(),
            expiry: 0,
        };
        observer.progress(crate::core::progress::CleanupProgress {
            stage: "confirmation".into(),
            completed: 0,
            total: 0,
            operation: Some(operation),
            account: Some(context.account.clone()),
            status: "simulation/preflight/confirmation".into(),
        });

        // Recheck state after quote latency; never transfer or burn a newly changed balance.
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
                super::transactions::send_instructions_observed(
                    self,
                    signer,
                    &[burn_instruction(&current, &owner)?],
                    limits,
                    Some((observer, &context)),
                )
                .await?
            }
            CleanupOperation::Close => {
                super::transactions::send_instructions_observed(
                    self,
                    signer,
                    &[close_instruction(&current, &owner)?],
                    limits,
                    Some((observer, &context)),
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

                // Validate the default input ATA before moving an auxiliary balance into it.
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
                super::swap::submit_with_instructions_observed(
                    self,
                    fresh,
                    signer,
                    limits,
                    &prefix,
                    &suffix,
                    Some((observer, &context)),
                )
                .await?
            }
        };
        Ok(receipt(self, operation, signature, &owner, &asset.account.address).await)
    }
}

#[cfg(test)]
mod accounting_tests {
    use super::*;
    #[test]
    fn confirmed_failure_fees_and_loaded_addresses_have_exact_signed_delta() {
        let value = json!({"transaction":{"message":{"accountKeys":["payer"]}},"meta":{"err":{"InstructionError":[0,"Custom"]},"preBalances":[18446744073709551615u64, 2039280],"postBalances":[18446744073709546615u64, 2039280],"loadedAddresses":{"writable":["source"],"readonly":[]}}});
        assert_eq!(
            transaction_accounting(&value, "payer", Some("source")),
            (Some(-5000), Some(0))
        );
        let mut value = value;
        value["meta"]["err"] = Value::Null;
        value["meta"]["postBalances"] = json!([18446744073709551615u64, 0]);
        assert_eq!(
            transaction_accounting(&value, "payer", Some("source")),
            (Some(0), Some(2039280))
        );
        value["meta"]["postBalances"] = Value::Null;
        assert_eq!(transaction_accounting(&value, "payer", None).0, None);
    }
}
