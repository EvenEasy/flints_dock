//! NFT burns use each standard's official instruction builder and the shared confirmed sender.
mod compressed;
mod core;
mod metadata;
use crate::{
    app::cleanup::*,
    app::swap::*,
    core::nft_cleanup::*,
    infra::solana::{self, scan::das::DasClient},
};
use compressed::compressed_burn;
use core::core_burn;
use metadata::metadata_burn;
use serde_json::{Value, json};
use solana_account::Account;
use solana_commitment_config::CommitmentConfig;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_signer::Signer;

fn invalid(message: impl ToString) -> SwapError {
    SwapError::InvalidRequest(message.to_string())
}
fn blocked(code: CleanupReasonCode, message: &str) -> SwapError {
    SwapError::NftBlocked {
        code,
        reason: message.into(),
    }
}
fn key(value: &str) -> Result<Pubkey> {
    value.parse().map_err(invalid)
}
fn io(error: impl ToString) -> SwapError {
    SwapError::Rpc(solana::safe_error(error))
}

/// Reuse the existing SPL executor while supplying a backend-owned, network-checked DAS reader.
pub struct SolanaCleanupExecutor<'a> {
    pub rpc: &'a RpcClient,
    pub das: Option<&'a DasClient>,
}

async fn account(rpc: &RpcClient, address: &Pubkey, program: Pubkey) -> Result<Account> {
    let account = rpc
        .get_account_with_commitment(address, CommitmentConfig::confirmed())
        .await
        .map_err(io)?
        .value
        .ok_or_else(|| invalid("Required NFT account is absent"))?;
    if account.owner != program {
        return Err(invalid("NFT account program owner changed"));
    }
    Ok(account)
}

impl CleanupExecutor for SolanaCleanupExecutor<'_> {
    async fn refresh(&self, address: &str, owner: &Pubkey) -> Result<Option<CleanupAsset>> {
        self.rpc.refresh(address, owner).await
    }
    async fn perform(
        &self,
        op: CleanupOperation,
        asset: &CleanupAsset,
        fresh: Option<PreparedSwap>,
        signer: &Keypair,
        limits: &SwapLimits,
    ) -> Result<OperationReceipt> {
        self.rpc.perform(op, asset, fresh, signer, limits).await
    }
    async fn perform_observed(
        &self,
        op: CleanupOperation,
        asset: &CleanupAsset,
        fresh: Option<PreparedSwap>,
        signer: &Keypair,
        limits: &SwapLimits,
        observer: &dyn crate::core::progress::CleanupObserver,
    ) -> Result<OperationReceipt> {
        self.rpc
            .perform_observed(op, asset, fresh, signer, limits, observer)
            .await
    }
    async fn failed_receipt(
        &self,
        op: CleanupOperation,
        signature: &str,
        owner: &Pubkey,
    ) -> Option<OperationReceipt> {
        self.rpc.failed_receipt(op, signature, owner).await
    }
    async fn prepare_nft(&self, target: &NftTarget, owner: &Pubkey) -> Result<PreparedNftBurn> {
        if target.owner != owner.to_string() {
            return Err(invalid("NFT owner does not match wallet"));
        }
        let prepared = match target.standard {
            NftStandard::TokenMetadata(kind) => metadata_burn(self.rpc, target, owner, kind).await,
            NftStandard::Core => core_burn(self.rpc, target, owner, self.das).await,
            NftStandard::Compressed => {
                compressed_burn(
                    self.rpc,
                    self.das.ok_or_else(|| {
                        invalid("DAS unavailable; compressed NFT proof cannot be fetched")
                    })?,
                    target,
                    owner,
                )
                .await
            }
        }?;
        // Reject oversized proof/account sets during preview, before offering an executable plan.
        let mut instructions = vec![solana::swap::compute_limit(1_400_000)];
        instructions.extend_from_slice(&prepared.instructions);
        let message = solana_message::v0::Message::try_compile(
            owner,
            &instructions,
            &[],
            solana_hash::Hash::default(),
        )
        .map_err(invalid)?;
        let message = solana_message::VersionedMessage::V0(message);
        let required = usize::from(message.header().num_required_signatures);
        if required != 1
            || bincode::serialize(&message).map_err(invalid)?.len() + 1 + 64 * required > 1232
        {
            return Err(blocked(
                CleanupReasonCode::UnsupportedAccountExtension,
                "NFT burn needs extra signers or lookup-table support for its proof/account set",
            ));
        }
        Ok(prepared)
    }
    async fn burn_nft(
        &self,
        target: &NftTarget,
        approved: &PreparedNftBurn,
        signer: &Keypair,
        limits: &SwapLimits,
        observer: &dyn crate::core::progress::CleanupObserver,
    ) -> Result<OperationReceipt> {
        let fresh = self.prepare_nft(target, &signer.pubkey()).await?;
        if fresh.edition_count > 0 {
            return Err(invalid(
                "Master NFT still has outstanding prints; its dependencies did not complete",
            ));
        }
        if fresh.identity != approved.identity {
            return Err(invalid(
                "NFT identity/state changed since preview; rebuild plan",
            ));
        }
        let context = crate::core::progress::PendingSubmission {
            wallet: signer.pubkey().to_string(),
            account: target.token_account.clone().unwrap_or(target.id.clone()),
            mint: target.mint.clone().unwrap_or(target.id.clone()),
            operation: CleanupOperation::Burn,
            signature: String::new(),
            expiry: 0,
        };
        let signature = solana::transactions::send_instructions_observed(
            self.rpc,
            signer,
            &fresh.instructions,
            limits,
            Some((observer, &context)),
        )
        .await?;
        // Metaplex closes the SPL source itself; Core leaves a burnt marker; cNFT has no token account.
        let verified: Result<bool> = async {
            Ok(match target.standard {
                NftStandard::TokenMetadata(_) => self
                    .rpc
                    .get_account_with_commitment(
                        &key(target.token_account.as_deref().expect("token target"))?,
                        CommitmentConfig::confirmed(),
                    )
                    .await
                    .map_err(io)?
                    .value
                    .is_none(),
                NftStandard::Core => self
                    .rpc
                    .get_account_with_commitment(&key(&target.id)?, CommitmentConfig::confirmed())
                    .await
                    .map_err(io)?
                    .value
                    .is_none_or(|a| {
                        a.data.first() == Some(&(mpl_core::types::Key::Uninitialized as u8))
                    }),
                NftStandard::Compressed => {
                    let das = self.das.expect("prepared DAS");
                    let mut burnt = false;
                    for _ in 0..4 {
                        let value = das.asset(&target.id).await?;
                        if value["burnt"] == true {
                            burnt = true;
                            break;
                        }
                        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    }
                    burnt
                }
            })
        }
        .await;
        let verified = verified.map_err(|error| SwapError::TransactionFailed {
            signature: signature.clone(),
            reason: format!("NFT burn confirmed; post-burn verification incomplete: {error}"),
        })?;
        if !verified {
            return Err(SwapError::TransactionFailed {
                signature,
                reason: "NFT burn confirmed but post-burn state is not yet verified".into(),
            });
        }
        let value: Option<Value> = self.rpc.send(solana_rpc_client_api::request::RpcRequest::GetTransaction, json!([signature, {"encoding":"json","commitment":"confirmed","maxSupportedTransactionVersion":0}])).await.ok().flatten();
        let (delta, reclaimed) = value
            .map(|v| {
                solana::cleanup::transaction_accounting(
                    &v,
                    &signer.pubkey().to_string(),
                    target.token_account.as_deref(),
                )
            })
            .unwrap_or((None, None));
        Ok(OperationReceipt {
            operation: CleanupOperation::Burn,
            signature,
            wallet_delta_lamports: delta,
            reclaimed_lamports: reclaimed,
        })
    }
}
