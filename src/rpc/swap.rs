use crate::{
    classification::classify_token_accounts,
    jupiter::swap::ApiInstruction,
    models::TokenProgram,
    scanner,
    swap::{PreparedSwap, Result, SwapError, SwapExecutor, SwapLimits, SwapRequest},
};
use base64::{Engine, engine::general_purpose::STANDARD};
use solana_client::{
    nonblocking::rpc_client::RpcClient,
    rpc_request::RpcRequest,
    rpc_response::{Response, RpcKeyedAccount},
};
use solana_hash::Hash;
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_message::{AddressLookupTableAccount, VersionedMessage, v0};
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use solana_transaction::versioned::VersionedTransaction;
use std::str::FromStr;

const COMPUTE_BUDGET: &str = "ComputeBudget111111111111111111111111111111";
const MAX_COMPUTE_UNITS: u32 = 1_400_000;
fn invalid(reason: impl ToString) -> SwapError {
    SwapError::InvalidResponse(reason.to_string())
}
fn rpc_error(reason: impl ToString) -> SwapError {
    SwapError::Rpc(reason.to_string())
}
fn address(text: &str) -> Result<Pubkey> {
    Pubkey::from_str(text).map_err(invalid)
}
fn instruction(api: &ApiInstruction) -> Result<Instruction> {
    Ok(Instruction {
        program_id: address(&api.program_id)?,
        accounts: api
            .accounts
            .iter()
            .map(|account| {
                Ok(AccountMeta {
                    pubkey: address(&account.pubkey)?,
                    is_signer: account.is_signer,
                    is_writable: account.is_writable,
                })
            })
            .collect::<Result<_>>()?,
        data: STANDARD.decode(&api.data).map_err(invalid)?,
    })
}
pub(crate) fn compute_limit(units: u32) -> Instruction {
    let mut data = vec![2];
    data.extend_from_slice(&units.to_le_bytes());
    Instruction {
        program_id: COMPUTE_BUDGET.parse().expect("constant program ID"),
        accounts: vec![],
        data,
    }
}

/// Compile API instructions in documented order. Explicitly retain WSOL cleanup.
/// The only allowed signer and fee payer is the selected local wallet.
pub fn signed_transaction(
    fresh: &PreparedSwap,
    signer: &Keypair,
    units: u32,
    limits: &SwapLimits,
) -> Result<VersionedTransaction> {
    signed_transaction_with_extras(fresh, signer, units, limits, &[], &[])
}

pub fn signed_transaction_with_extras(
    fresh: &PreparedSwap,
    signer: &Keypair,
    units: u32,
    limits: &SwapLimits,
    prefix: &[Instruction],
    suffix: &[Instruction],
) -> Result<VersionedTransaction> {
    if signer.pubkey() != fresh.request.wallet {
        return Err(SwapError::InvalidRequest("keypair/wallet mismatch".into()));
    }
    if units == 0 || units > MAX_COMPUTE_UNITS {
        return Err(invalid("invalid compute limit"));
    }
    let build = &fresh.build;
    if build.tip_instruction.is_some() {
        return Err(invalid("unsolicited tip instruction"));
    }
    let mut instructions = vec![compute_limit(units)];
    if build.compute_budget_instructions.len() > 1 {
        return Err(invalid("unexpected compute budget instructions"));
    }
    for api in &build.compute_budget_instructions {
        let ix = instruction(api)?;
        if ix.program_id.to_string() != COMPUTE_BUDGET
            || ix.data.len() != 9
            || ix.data[0] != 3
            || !ix.accounts.is_empty()
        {
            return Err(invalid("expected compute unit price instruction"));
        }
        let price = u64::from_le_bytes(ix.data[1..].try_into().map_err(invalid)?);
        let fee = (u128::from(price) * u128::from(units)).div_ceil(1_000_000);
        if fee > u128::from(limits.max_priority_fee_lamports) {
            return Err(SwapError::InvalidRequest(
                "route exceeds --max-priority-fee-lamports".into(),
            ));
        }
        instructions.push(ix);
    }
    instructions.extend_from_slice(prefix);
    for api in build
        .setup_instructions
        .iter()
        .chain(std::iter::once(&build.swap_instruction))
        .chain(build.cleanup_instruction.iter())
        .chain(&build.other_instructions)
    {
        let ix = instruction(api)?;
        if ix.program_id.to_string() == COMPUTE_BUDGET {
            return Err(invalid("unexpected compute instruction outside budget"));
        }
        if ix
            .accounts
            .iter()
            .any(|account| account.is_signer && account.pubkey != signer.pubkey())
        {
            return Err(invalid("route requires another signer"));
        }
        instructions.push(ix);
    }
    instructions.extend_from_slice(suffix);
    let mut tables = Vec::new();
    if let Some(mapping) = &build.addresses_by_lookup_table_address {
        for (key, addresses) in mapping {
            tables.push(AddressLookupTableAccount {
                key: address(key)?,
                addresses: addresses
                    .iter()
                    .map(|s| address(s))
                    .collect::<Result<_>>()?,
            });
        }
    }
    let message = v0::Message::try_compile(
        &signer.pubkey(),
        &instructions,
        &tables,
        Hash::new_from_array(build.blockhash_with_metadata.blockhash),
    )
    .map_err(invalid)?;
    let tx =
        VersionedTransaction::try_new(VersionedMessage::V0(message), &[signer]).map_err(invalid)?;
    if bincode::serialize(&tx).map_err(invalid)?.len() > 1232 {
        return Err(invalid("swap transaction exceeds Solana packet size"));
    }
    Ok(tx)
}

impl SwapExecutor for RpcClient {
    async fn check_input(&self, request: &SwapRequest) -> Result<()> {
        // Restrict discovery to the exact input mint, reusing existing raw decoders
        // and classifier instead of scanning/pricing the whole portfolio.
        let response: Response<Vec<RpcKeyedAccount>> = self
            .send(
                RpcRequest::GetTokenAccountsByOwner,
                serde_json::json!([request.wallet.to_string(), {"mint":request.mint.to_string()},
                {"encoding":"base64","commitment":"confirmed"}]),
            )
            .await
            .map_err(rpc_error)?;
        let mut accounts = Vec::new();
        for account in response.value {
            let program = if account.account.owner == TokenProgram::Legacy.id().to_string() {
                TokenProgram::Legacy
            } else if account.account.owner == TokenProgram::Token2022.id().to_string() {
                TokenProgram::Token2022
            } else {
                return Err(rpc_error("unexpected token program"));
            };
            let account = scanner::tokens::parse_account(&account, program, &request.wallet)
                .map_err(rpc_error)?;
            if account.mint != request.mint.to_string() {
                return Err(rpc_error("RPC returned a different mint"));
            }
            accounts.push(account);
        }
        if accounts
            .iter()
            .map(|a| u128::from(a.raw_amount))
            .sum::<u128>()
            < u128::from(request.raw_amount)
        {
            return Err(SwapError::InsufficientFunds(
                "wallet does not hold the requested raw amount".into(),
            ));
        }
        let mints = scanner::metadata::get_mints(self, &mut accounts).await;
        if !mints.status.is_complete() {
            return Err(rpc_error("cannot verify mint semantics"));
        }
        let metadata = scanner::nft::get_metadata(self, &accounts, &mints.items).await;
        if !metadata.status.is_complete() {
            return Err(rpc_error("cannot verify input metadata"));
        }
        let assets = classify_token_accounts(&accounts, &mints.items, &metadata.items);
        if assets.is_empty() || assets.iter().any(|asset| !asset.kind.is_fungible()) {
            return Err(SwapError::InvalidRequest("execution supports verified fungible assets only; NFT/Unknown liquidation is excluded".into()));
        }
        Ok(())
    }

    async fn submit(
        &self,
        fresh: PreparedSwap,
        signer: &Keypair,
        limits: &SwapLimits,
    ) -> Result<String> {
        submit_with_instructions(self, fresh, signer, limits, &[], &[]).await
    }
}

pub async fn submit_with_instructions(
    rpc: &RpcClient,
    fresh: PreparedSwap,
    signer: &Keypair,
    limits: &SwapLimits,
    prefix: &[Instruction],
    suffix: &[Instruction],
) -> Result<String> {
    fresh.ensure_fresh(limits)?;
    let expiry = fresh.build.blockhash_with_metadata.last_valid_block_height;
    if rpc.get_block_height().await.map_err(rpc_error)? > expiry {
        return Err(SwapError::Expired);
    }
    let initial =
        signed_transaction_with_extras(&fresh, signer, MAX_COMPUTE_UNITS, limits, prefix, suffix)?;
    let units = super::transactions::simulate(rpc, &initial).await?;
    let transaction =
        signed_transaction_with_extras(&fresh, signer, units, limits, prefix, suffix)?;
    if rpc.get_block_height().await.map_err(rpc_error)? > expiry {
        return Err(SwapError::Expired);
    }
    fresh.ensure_fresh(limits)?;
    super::transactions::send_confirm(rpc, &transaction, expiry, limits).await
}
