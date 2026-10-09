//! Desktop lifecycle and approval policy. All on-chain actions remain in core.
use crate::{dto::cleanup::*, error::AppError, journal::Journal};
use dock_flints_core::{
    core::{cleanup::*, progress::*},
    infra::wallet::WalletIdentity,
};
use solana_signer::Signer;
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

pub fn new_id() -> String {
    solana_keypair::Keypair::new().pubkey().to_string()
}
pub struct Session {
    pub id: String,
    pub identity: Arc<WalletIdentity>,
}
pub struct StoredPlan {
    pub session_id: String,
    pub revision: u64,
    pub network: String,
    pub expires: u64,
    pub core: Arc<CleanupPlan>,
    pub options: CleanupOptions,
    pub dto: CleanupPlanDto,
    pub job_id: Option<String>,
}
#[derive(Default)]
pub struct DesktopStore {
    pub session: Option<Session>,
    pub revision: u64,
    pub plans: BTreeMap<String, StoredPlan>,
    pub jobs: BTreeMap<String, CleanupJobDto>,
    pub active_job: Option<String>,
}
impl DesktopStore {
    /// Reject session changes, stale selection/expiry and read-only execution independently of UI.
    pub fn approve(
        &mut self,
        request: &ExecuteRequest,
        network: &str,
    ) -> Result<
        (
            String,
            Arc<WalletIdentity>,
            Arc<CleanupPlan>,
            CleanupOptions,
        ),
        AppError,
    > {
        let session = self
            .session
            .as_ref()
            .filter(|s| s.id == request.session_id)
            .ok_or_else(|| AppError::cleanup("Wallet session changed"))?;
        session
            .identity
            .signer()
            .map_err(|_| AppError::cleanup("Read-only wallet cannot execute cleanup"))?;
        let plan = self
            .plans
            .get_mut(&request.plan_id)
            .ok_or_else(|| AppError::cleanup("Plan unavailable; prepare again"))?;
        if plan.session_id != session.id
            || plan.core.wallet != session.identity.address.to_string()
            || plan.network != network
        {
            return Err(AppError::cleanup("Plan wallet/session/network mismatch"));
        }
        if let Some(id) = &plan.job_id {
            return Ok((
                id.clone(),
                session.identity.clone(),
                plan.core.clone(),
                plan.options.clone(),
            ));
        }
        if self.active_job.is_some() {
            return Err(AppError::cleanup("Another cleanup is running"));
        }
        if plan.revision != self.revision
            || plan.expires <= dock_flints_core::core::categories::now()
        {
            return Err(AppError::cleanup(
                "Plan expired or selection changed; prepare again",
            ));
        }
        if !plan.dto.can_execute || plan.core.summary.accounts_to_close == 0 {
            return Err(AppError::cleanup(
                "No selected eligible accounts; nothing will be submitted",
            ));
        }
        if !request.approval.close
            || (plan.core.summary.swappable > 0 && !request.approval.swap)
            || (plan.core.summary.burnable > 0 && !request.approval.burn)
        {
            return Err(AppError::cleanup(
                "Explicit approval of every planned action is required",
            ));
        }
        let id = new_id();
        plan.job_id = Some(id.clone());
        self.active_job = Some(id.clone());
        self.jobs.insert(
            id.clone(),
            CleanupJobDto {
                job_id: id.clone(),
                session_id: session.id.clone(),
                network: network.into(),
                status: "running".into(),
                sequence: 0,
                progress: vec![],
                report: None,
                error: None,
            },
        );
        Ok((
            id,
            session.identity.clone(),
            plan.core.clone(),
            plan.options.clone(),
        ))
    }
}

/// Channel loss does not cancel a sent transaction; snapshots remain queryable in Rust.
pub struct JobObserver {
    pub store: Arc<Mutex<DesktopStore>>,
    pub journal: Arc<Journal>,
    pub channel: Option<tauri::ipc::Channel<ProgressEvent>>,
    pub job_id: String,
    pub session_id: String,
    pub network: String,
}
impl CleanupObserver for JobObserver {
    fn progress(&self, progress: CleanupProgress) {
        if progress.status == "finished" {
            return;
        }
        if let Ok(mut store) = self.store.lock()
            && let Some(job) = store.jobs.get_mut(&self.job_id)
        {
            job.sequence += 1;
            let event = ProgressEvent {
                job_id: self.job_id.clone(),
                session_id: self.session_id.clone(),
                sequence: job.sequence,
                progress,
            };
            if job.progress.len() >= 1000 {
                job.progress.remove(0);
            }
            job.progress.push(event.clone());
            if let Some(channel) = &self.channel {
                let _ = channel.send(event);
            }
        }
    }
    fn before_send(
        &self,
        submission: PendingSubmission,
    ) -> Result<(), dock_flints_core::core::error::SwapError> {
        self.journal
            .submit(&self.job_id, &self.network, submission)
            .map_err(|e| dock_flints_core::core::error::SwapError::InvalidRequest(e.message))
    }
    // Keep pending until metadata is reconciled: a crash immediately after confirmation is safe.
}

/// Check signature history before any new execution; never resend an unresolved transaction.
pub async fn reconcile(
    rpc: &solana_rpc_client::nonblocking::rpc_client::RpcClient,
    journal: &Journal,
    network: &str,
    owner: &str,
) -> Result<(), AppError> {
    use solana_rpc_client_api::request::RpcRequest;
    for record in journal.pending(network, owner)? {
        let signature = record
            .signature
            .parse()
            .map_err(|_| AppError::cleanup("Invalid journal signature"))?;
        let status = rpc
            .get_signature_statuses_with_history(&[signature])
            .await
            .map_err(|_| {
                AppError::cleanup("Cannot reconcile submitted signatures; execution blocked")
            })?;
        let confirmed = status.value.first().and_then(|s| s.as_ref()).filter(|s| {
            s.satisfies_commitment(solana_commitment_config::CommitmentConfig::confirmed())
        });
        let Some(status) = confirmed else {
            return Err(AppError::cleanup(
                "A submitted signature is unresolved. Execution blocked; no transaction resent",
            ));
        };
        let value: Option<serde_json::Value> = rpc.send(RpcRequest::GetTransaction, serde_json::json!([record.signature, {"encoding":"json","commitment":"confirmed","maxSupportedTransactionVersion":0}])).await.map_err(|_| AppError::cleanup("Confirmed signature metadata unavailable; reconciliation pending"))?;
        let delta = value
            .as_ref()
            .and_then(|v| {
                dock_flints_core::infra::solana::cleanup::transaction_accounting(v, owner, None).0
            })
            .map(|n| n.to_string());
        if delta.is_none() {
            return Err(AppError::cleanup(
                "Confirmed signature metadata unavailable; reconciliation pending",
            ));
        }
        journal.resolve(
            &record.signature,
            if status.err.is_some() {
                "failed"
            } else {
                "confirmed"
            },
            delta,
        )?;
    }
    Ok(())
}

/// The actual genesis hash, rather than URL spelling, defines provider scope and approval network.
pub async fn network(
    rpc: &solana_rpc_client::nonblocking::rpc_client::RpcClient,
) -> Result<(String, bool), AppError> {
    let hash = rpc
        .get_genesis_hash()
        .await
        .map_err(|_| AppError::cleanup("Cannot establish RPC network"))?
        .to_string();
    let mainnet = hash == "5eykt4UsFv8P8NJdTREpY1vzqKqZKvdpKuc147dw2N9d";
    Ok((hash, mainnet))
}

/// Read-only stages have their own scoped channel, independent of an execution job.
pub struct ReadObserver {
    pub channel: Option<tauri::ipc::Channel<ProgressEvent>>,
    pub job_id: String,
    pub session_id: String,
    pub sequence: std::sync::atomic::AtomicU64,
}
impl CleanupObserver for ReadObserver {
    fn progress(&self, progress: CleanupProgress) {
        if let Some(channel) = &self.channel {
            let _ = channel.send(ProgressEvent {
                job_id: self.job_id.clone(),
                session_id: self.session_id.clone(),
                sequence: self
                    .sequence
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
                    + 1,
                progress,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dock_flints_core::core::{ScanStatus, categories::now};
    fn store(signing: bool) -> DesktopStore {
        let identity = if signing {
            WalletIdentity::from_seed("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=").unwrap()
        } else {
            WalletIdentity::read_only(solana_pubkey::Pubkey::default())
        };
        let owner = identity.address.to_string();
        let options = CleanupOptions::default();
        let dto = CleanupPlanDto {
            plan_id: "plan".into(),
            session_id: "session".into(),
            revision: 1,
            network: "network".into(),
            expires_at: (now() + 100).to_string(),
            entries: vec![],
            can_execute: true,
            requires_burn: false,
            swap_count: 0,
            burn_count: 0,
            close_count: 1,
            estimated_swap_lamports: "0".into(),
            estimated_reclaimed_lamports: "2039280".into(),
            max_price_impact_bps: 100,
            max_priority_fee_lamports: "1000000".into(),
            slippage_bps: 50,
        };
        let core = CleanupPlan {
            wallet: owner,
            discovery_status: ScanStatus::Complete,
            entries: vec![],
            unparsed_accounts: vec![],
            summary: CleanupSummary {
                accounts_to_close: 1,
                ..Default::default()
            },
            selection: Default::default(),
        };
        DesktopStore {
            session: Some(Session {
                id: "session".into(),
                identity: Arc::new(identity),
            }),
            revision: 1,
            plans: BTreeMap::from([(
                "plan".into(),
                StoredPlan {
                    session_id: "session".into(),
                    revision: 1,
                    network: "network".into(),
                    expires: now() + 100,
                    core: Arc::new(core),
                    options,
                    dto,
                    job_id: None,
                },
            )]),
            ..Default::default()
        }
    }
    fn approval() -> ExecuteRequest {
        ExecuteRequest {
            session_id: "session".into(),
            plan_id: "plan".into(),
            approval: Approval {
                swap: true,
                burn: true,
                close: true,
            },
        }
    }
    #[test]
    fn read_only_none_stale_expiry_and_mismatch_never_claim_a_job() {
        let request = approval();
        assert!(store(false).approve(&request, "network").is_err());
        for kind in [
            "none",
            "selection",
            "expiry",
            "wallet",
            "session",
            "network",
            "approval",
        ] {
            let mut store = store(true);
            let mut request = approval();
            match kind {
                "none" => store.plans.get_mut("plan").unwrap().dto.can_execute = false,
                "selection" => store.revision = 2,
                "expiry" => store.plans.get_mut("plan").unwrap().expires = 0,
                "wallet" => {
                    Arc::make_mut(&mut store.plans.get_mut("plan").unwrap().core).wallet =
                        "other".into()
                }
                "session" => request.session_id = "other".into(),
                "network" => store.plans.get_mut("plan").unwrap().network = "other".into(),
                _ => request.approval.close = false,
            }
            assert!(store.approve(&request, "network").is_err(), "{kind}");
            assert!(store.active_job.is_none());
        }
    }
    #[test]
    fn duplicate_execute_claims_exactly_one_job_and_burn_needs_approval() {
        let mut store = store(true);
        let request = approval();
        let first = store.approve(&request, "network").unwrap().0;
        let duplicate = store.approve(&request, "network").unwrap().0;
        assert_eq!(first, duplicate);
        assert_eq!(store.jobs.len(), 1);
        let mut store = self::store(true);
        Arc::make_mut(&mut store.plans.get_mut("plan").unwrap().core)
            .summary
            .burnable = 1;
        let mut request = approval();
        request.approval.burn = false;
        assert!(store.approve(&request, "network").is_err());
    }
}
