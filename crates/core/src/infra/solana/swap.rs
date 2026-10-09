use crate::{
    app::swap::{PreparedSwap, Result, SwapError, SwapExecutor, SwapLimits, SwapRequest},
    core::TokenProgram,
    core::classification::classify_token_accounts,
    infra::solana::scan as scanner,
};
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_message::{VersionedMessage, v0};
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_rpc_client_api::{
    request::RpcRequest,
    response::{Response, RpcKeyedAccount},
};
use solana_signer::Signer;
use solana_transaction::versioned::VersionedTransaction;

const COMPUTE_BUDGET: &str = "ComputeBudget111111111111111111111111111111";
const MAX_COMPUTE_UNITS: u32 = 1_400_000;
fn invalid(reason: impl ToString) -> SwapError {
    SwapError::InvalidResponse(reason.to_string())
}
fn rpc_error(reason: impl ToString) -> SwapError {
    SwapError::Rpc(super::safe_error(reason))
}

/// Build the compute-budget instruction for the chosen unit allowance.
pub(crate) fn compute_limit(units: u32) -> Instruction {
    let mut data = vec![2];
    data.extend_from_slice(&units.to_le_bytes());
    Instruction {
        program_id: COMPUTE_BUDGET.parse().expect("constant program ID"),
        accounts: vec![],
        data,
    }
}

/// Return a locally signed versioned swap transaction without source-account additions.
/// Enforces signer, compute, fee and packet-size limits and retains native SOL cleanup.
pub fn signed_transaction(
    fresh: &PreparedSwap,
    signer: &Keypair,
    units: u32,
    limits: &SwapLimits,
) -> Result<VersionedTransaction> {
    signed_transaction_with_extras(fresh, signer, units, limits, &[], &[])
}

/// Return a signed versioned swap with local `prefix` and `suffix` instructions.
/// The caller supplies account-scoped preparation and cleanup; provider instructions are
/// checked for extra signers, unsolicited tips, compute pricing and packet-size limits.
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

    // Validate the compute price instruction and enforce the priority-fee cap.
    for api in &build.compute_budget_instructions {
        let ix = api.clone();
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

    // Preserve setup, swap and native SOL cleanup order while rejecting extra signers.
    for api in build
        .setup_instructions
        .iter()
        .chain(std::iter::once(&build.swap_instruction))
        .chain(build.cleanup_instruction.iter())
        .chain(&build.other_instructions)
    {
        let ix = api.clone();
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
    let message = v0::Message::try_compile(
        &signer.pubkey(),
        &instructions,
        &build.lookup_tables,
        build.blockhash,
    )
    .map_err(invalid)?;
    let tx =
        VersionedTransaction::try_new(VersionedMessage::V0(message), &[signer]).map_err(invalid)?;

    // Reject oversized packets before simulation or submission.
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

        // Verify mint and NFT semantics before allowing a fungible swap.
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

/// Simulate and confirm a fresh swap with atomic source-account instructions.
/// Returns its signature on confirmation; expiry or simulation errors occur before submission,
/// while send/confirmation ambiguity preserves the signature in `Uncertain`.
pub async fn submit_with_instructions_observed(
    rpc: &RpcClient,
    fresh: PreparedSwap,
    signer: &Keypair,
    limits: &SwapLimits,
    prefix: &[Instruction],
    suffix: &[Instruction],
    observer: Option<(
        &dyn crate::core::progress::CleanupObserver,
        &crate::core::progress::PendingSubmission,
    )>,
) -> Result<String> {
    fresh.ensure_fresh(limits)?;
    let expiry = fresh.build.last_valid_block_height;
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
    super::transactions::send_confirm_observed(rpc, &transaction, expiry, limits, observer).await
}

/// Preserve the CLI swap boundary without installing host persistence.
pub async fn submit_with_instructions(
    rpc: &RpcClient,
    fresh: PreparedSwap,
    signer: &Keypair,
    limits: &SwapLimits,
    prefix: &[Instruction],
    suffix: &[Instruction],
) -> Result<String> {
    submit_with_instructions_observed(rpc, fresh, signer, limits, prefix, suffix, None).await
}
