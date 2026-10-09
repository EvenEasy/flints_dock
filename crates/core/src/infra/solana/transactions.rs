use crate::app::swap::{Result, SwapError, SwapLimits};
use solana_commitment_config::CommitmentConfig;
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_message::{VersionedMessage, v0};
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_rpc_client_api::config::{RpcSendTransactionConfig, RpcSimulateTransactionConfig};
use solana_signer::Signer;
use solana_transaction::versioned::VersionedTransaction;
use std::time::Duration;

/// Submit the signed transaction and return its confirmed signature.
/// `expiry` is the last valid block height. A timeout, transport failure or unobserved
/// expiry returns `Uncertain` with the original signature; no new transaction is built.
pub async fn send_confirm_observed(
    rpc: &RpcClient,
    transaction: &VersionedTransaction,
    expiry: u64,
    limits: &SwapLimits,
    observer: Option<(
        &dyn crate::core::progress::CleanupObserver,
        &crate::core::progress::PendingSubmission,
    )>,
) -> Result<String> {
    // Keep our locally computed signature even if an RPC timeout obscures send success.
    let signature = transaction.signatures[0];
    let uncertain = |reason: String| SwapError::Uncertain {
        signature: signature.to_string(),
        reason,
    };

    if let Some((observer, context)) = observer {
        let mut submission = context.clone();
        submission.signature = signature.to_string();
        submission.expiry = expiry;
        observer.before_send(submission)?;
    }

    // Submit the existing signed transaction with preflight enabled and bounded RPC retries.
    let sent = rpc
        .send_transaction_with_config(
            transaction,
            RpcSendTransactionConfig {
                skip_preflight: false,
                preflight_commitment: Some(CommitmentConfig::confirmed().commitment),
                max_retries: Some(2),
                ..Default::default()
            },
        )
        .await
        .map_err(|e| uncertain(super::safe_error(e)))?;
    if sent != signature {
        return Err(uncertain("RPC returned a different signature".into()));
    }

    if let Some((observer, context)) = observer {
        observer.progress(crate::core::progress::CleanupProgress {
            stage: "confirmation".into(),
            completed: 0,
            total: 0,
            operation: Some(context.operation),
            account: Some(context.account.clone()),
            status: "running".into(),
        });
    }

    // Poll the original signature until confirmed success, confirmed failure or expiry.
    let confirmation = async {
        loop {
            let statuses = rpc
                .get_signature_statuses_with_history(&[signature])
                .await
                .map_err(|e| uncertain(super::safe_error(e)))?;

            // Accept a result only at the requested commitment level.
            if let Some(Some(status)) = statuses.value.first()
                && status.satisfies_commitment(CommitmentConfig::confirmed())
            {
                if let Some((observer, _)) = observer {
                    observer.confirmed(&signature.to_string(), status.err.is_some());
                }
                if let Some(error) = &status.err {
                    return Err(SwapError::TransactionFailed {
                        signature: signature.to_string(),
                        reason: error.to_string(),
                    });
                }
                return Ok(signature.to_string());
            }

            // Stop polling when the blockhash expires without observed confirmation.
            if rpc
                .get_block_height()
                .await
                .map_err(|e| uncertain(super::safe_error(e)))?
                > expiry
            {
                return Err(uncertain(
                    "blockhash expired without observed confirmation".into(),
                ));
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    };
    tokio::time::timeout(limits.confirmation_timeout, confirmation)
        .await
        .map_err(|_| uncertain("confirmation timeout".into()))?
}

/// Simulate a signed transaction and return a compute-unit limit with a 20% margin.
/// Caps the result at 1,400,000 units and uses that cap when consumption is unavailable.
/// RPC or execution failures return `Simulation` before any submission.
pub async fn simulate(rpc: &RpcClient, transaction: &VersionedTransaction) -> Result<u32> {
    let result = rpc
        .simulate_transaction_with_config(
            transaction,
            RpcSimulateTransactionConfig {
                sig_verify: true,
                commitment: Some(CommitmentConfig::confirmed()),
                ..Default::default()
            },
        )
        .await
        .map_err(|e| SwapError::Simulation(super::safe_error(e)))?
        .value;
    if let Some(error) = result.err {
        return Err(SwapError::Simulation(error.to_string()));
    }
    Ok(result
        .units_consumed
        .map(|units| units.saturating_mul(120).div_ceil(100).clamp(1, 1_400_000) as u32)
        .unwrap_or(1_400_000))
}

/// Build, sign, simulate and confirm local burn or close instructions.
/// Returns the confirmed signature and applies the same preflight, expiry and uncertainty
/// rules as swaps. Instruction eligibility must be checked by the caller.
pub async fn send_instructions_observed(
    rpc: &RpcClient,
    signer: &Keypair,
    instructions: &[Instruction],
    limits: &SwapLimits,
    observer: Option<(
        &dyn crate::core::progress::CleanupObserver,
        &crate::core::progress::PendingSubmission,
    )>,
) -> Result<String> {
    let (hash, expiry) = rpc
        .get_latest_blockhash_with_commitment(CommitmentConfig::confirmed())
        .await
        .map_err(|e| SwapError::Rpc(super::safe_error(e)))?;
    let compile = |units| -> Result<VersionedTransaction> {
        let mut all = vec![super::swap::compute_limit(units)];
        all.extend_from_slice(instructions);
        let message = v0::Message::try_compile(&signer.pubkey(), &all, &[], hash)
            .map_err(|e| SwapError::InvalidRequest(e.to_string()))?;
        VersionedTransaction::try_new(VersionedMessage::V0(message), &[signer])
            .map_err(|e| SwapError::InvalidRequest(e.to_string()))
    };

    // Use simulation consumption to rebuild the same operation with an appropriate compute limit.
    let initial = compile(1_400_000)?;
    let units = simulate(rpc, &initial).await?;
    let transaction = compile(units)?;
    if rpc
        .get_block_height()
        .await
        .map_err(|e| SwapError::Rpc(super::safe_error(e)))?
        > expiry
    {
        return Err(SwapError::Expired);
    }
    send_confirm_observed(rpc, &transaction, expiry, limits, observer).await
}

/// CLI-compatible submission without a host journal.
pub async fn send_confirm(
    rpc: &RpcClient,
    transaction: &VersionedTransaction,
    expiry: u64,
    limits: &SwapLimits,
) -> Result<String> {
    send_confirm_observed(rpc, transaction, expiry, limits, None).await
}
/// CLI-compatible local instruction execution.
pub async fn send_instructions(
    rpc: &RpcClient,
    signer: &Keypair,
    instructions: &[Instruction],
    limits: &SwapLimits,
) -> Result<String> {
    send_instructions_observed(rpc, signer, instructions, limits, None).await
}
