use crate::swap::{Result, SwapError, SwapLimits};
use solana_client::{
    nonblocking::rpc_client::RpcClient,
    rpc_config::{RpcSendTransactionConfig, RpcSimulateTransactionConfig},
};
use solana_commitment_config::CommitmentConfig;
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_message::{VersionedMessage, v0};
use solana_signer::Signer;
use solana_transaction::versioned::VersionedTransaction;
use std::time::Duration;

pub async fn send_confirm(
    rpc: &RpcClient,
    transaction: &VersionedTransaction,
    expiry: u64,
    limits: &SwapLimits,
) -> Result<String> {
    // Keep our locally computed signature even if an RPC timeout obscures send success.
    let signature = transaction.signatures[0];
    let uncertain = |reason: String| SwapError::Uncertain {
        signature: signature.to_string(),
        reason,
    };
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
        .map_err(|e| uncertain(e.to_string()))?;
    if sent != signature {
        return Err(uncertain("RPC returned a different signature".into()));
    }
    let confirmation = async {
        loop {
            let statuses = rpc
                .get_signature_statuses_with_history(&[signature])
                .await
                .map_err(|e| uncertain(e.to_string()))?;
            if let Some(Some(status)) = statuses.value.first()
                && status.satisfies_commitment(CommitmentConfig::confirmed())
            {
                if let Some(error) = &status.err {
                    return Err(SwapError::TransactionFailed {
                        signature: signature.to_string(),
                        reason: error.to_string(),
                    });
                }
                return Ok(signature.to_string());
            }
            if rpc
                .get_block_height()
                .await
                .map_err(|e| uncertain(e.to_string()))?
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
        .map_err(|e| SwapError::Simulation(e.to_string()))?
        .value;
    if let Some(error) = result.err {
        return Err(SwapError::Simulation(error.to_string()));
    }
    Ok(result
        .units_consumed
        .map(|units| units.saturating_mul(120).div_ceil(100).clamp(1, 1_400_000) as u32)
        .unwrap_or(1_400_000))
}

/// Burn and close use the same simulation, preflight and confirmation rules as swaps.
pub async fn send_instructions(
    rpc: &RpcClient,
    signer: &Keypair,
    instructions: &[Instruction],
    limits: &SwapLimits,
) -> Result<String> {
    let (hash, expiry) = rpc
        .get_latest_blockhash_with_commitment(CommitmentConfig::confirmed())
        .await
        .map_err(|e| SwapError::Rpc(e.to_string()))?;
    let compile = |units| -> Result<VersionedTransaction> {
        let mut all = vec![super::swap::compute_limit(units)];
        all.extend_from_slice(instructions);
        let message = v0::Message::try_compile(&signer.pubkey(), &all, &[], hash)
            .map_err(|e| SwapError::InvalidRequest(e.to_string()))?;
        VersionedTransaction::try_new(VersionedMessage::V0(message), &[signer])
            .map_err(|e| SwapError::InvalidRequest(e.to_string()))
    };
    let initial = compile(1_400_000)?;
    let units = simulate(rpc, &initial).await?;
    let transaction = compile(units)?;
    if rpc
        .get_block_height()
        .await
        .map_err(|e| SwapError::Rpc(e.to_string()))?
        > expiry
    {
        return Err(SwapError::Expired);
    }
    send_confirm(rpc, &transaction, expiry, limits).await
}
