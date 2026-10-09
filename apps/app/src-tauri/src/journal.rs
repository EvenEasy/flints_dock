//! Durable nonsensitive signatures. A signed transaction is persisted before the first send.
use crate::error::AppError;
use dock_flints_core::core::progress::PendingSubmission;
use serde::{Deserialize, Serialize};
use std::{io::Write, path::PathBuf, sync::Mutex};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Record {
    pub job_id: String,
    pub network: String,
    pub wallet: String,
    pub account: String,
    pub mint: String,
    pub operation: String,
    pub signature: String,
    pub expiry: String,
    pub state: String,
    pub delta: Option<String>,
}
/// Only signatures/identities/accounting are retained; neither key material nor transaction bytes.
pub struct Journal {
    path: PathBuf,
    records: Mutex<Vec<Record>>,
}
impl Journal {
    pub fn open(path: PathBuf) -> Result<Self, AppError> {
        let records = match std::fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(|_| {
                AppError::cleanup("Signature journal is invalid; execution disabled until reviewed")
            })?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => vec![],
            Err(_) => return Err(AppError::cleanup("Cannot read signature journal")),
        };
        Ok(Self {
            path,
            records: Mutex::new(records),
        })
    }
    fn save(&self, records: &[Record]) -> Result<(), AppError> {
        let parent = self
            .path
            .parent()
            .ok_or_else(|| AppError::cleanup("Invalid journal path"))?;
        std::fs::create_dir_all(parent)
            .map_err(|_| AppError::cleanup("Cannot create journal directory"))?;
        let temporary = self.path.with_extension("tmp");
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&temporary).map_err(|_| {
            AppError::cleanup("Cannot persist signature journal; transaction not sent")
        })?;
        file.write_all(
            &serde_json::to_vec(records)
                .map_err(|_| AppError::cleanup("Cannot serialize journal"))?,
        )
        .and_then(|_| file.sync_all())
        .map_err(|_| AppError::cleanup("Cannot sync signature journal"))?;
        std::fs::rename(&temporary, &self.path)
            .map_err(|_| AppError::cleanup("Cannot commit signature journal"))?;
        std::fs::File::open(parent)
            .and_then(|f| f.sync_all())
            .map_err(|_| AppError::cleanup("Cannot sync journal directory"))?;
        Ok(())
    }
    /// An OS file lock prevents simultaneous app processes from submitting against the same journal.
    /// The returned File (not a MutexGuard) remains owned by the async execution scope.
    pub fn execution_lock(&self) -> Result<std::fs::File, AppError> {
        let parent = self
            .path
            .parent()
            .ok_or_else(|| AppError::cleanup("Invalid journal path"))?;
        std::fs::create_dir_all(parent)
            .map_err(|_| AppError::cleanup("Cannot create journal directory"))?;
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .open(self.path.with_extension("lock"))
            .map_err(|_| AppError::cleanup("Cannot open execution lock"))?;
        file.try_lock()
            .map_err(|_| AppError::cleanup("Cleanup is running in another app instance"))?;
        // Reload after acquiring the process lock; another instance may have submitted since startup.
        if let Ok(bytes) = std::fs::read(&self.path) {
            let current = serde_json::from_slice(&bytes)
                .map_err(|_| AppError::cleanup("Invalid signature journal"))?;
            *self
                .records
                .lock()
                .map_err(|_| AppError::cleanup("Journal unavailable"))? = current;
        }
        Ok(file)
    }

    /// Sum only this job's confirmed transaction metadata, never a whole-wallet balance interval.
    pub fn accounting(&self, job: &str) -> Result<(i128, bool), AppError> {
        let records = self
            .records
            .lock()
            .map_err(|_| AppError::cleanup("Journal unavailable"))?;
        let mut total = 0i128;
        let mut complete = true;
        for record in records.iter().filter(|record| record.job_id == job) {
            match record
                .delta
                .as_ref()
                .and_then(|value| value.parse::<i128>().ok())
            {
                Some(delta) if record.state != "pending" => {
                    total = total
                        .checked_add(delta)
                        .ok_or_else(|| AppError::cleanup("Accounting total overflow"))?;
                }
                _ => complete = false,
            }
        }
        Ok((total, complete))
    }

    pub fn pending(&self, network: &str, owner: &str) -> Result<Vec<Record>, AppError> {
        Ok(self
            .records
            .lock()
            .map_err(|_| AppError::cleanup("Journal unavailable"))?
            .iter()
            .filter(|r| r.network == network && r.wallet == owner && r.state == "pending")
            .cloned()
            .collect())
    }
    pub fn submit(
        &self,
        job: &str,
        network: &str,
        value: PendingSubmission,
    ) -> Result<(), AppError> {
        let mut records = self
            .records
            .lock()
            .map_err(|_| AppError::cleanup("Journal unavailable"))?;
        if records.iter().any(|r| r.signature == value.signature) {
            return Err(AppError::cleanup("Signature already journaled; no resend"));
        }
        let record = Record {
            job_id: job.into(),
            network: network.into(),
            wallet: value.wallet,
            account: value.account,
            mint: value.mint,
            operation: format!("{:?}", value.operation),
            signature: value.signature,
            expiry: value.expiry.to_string(),
            state: "pending".into(),
            delta: None,
        };
        records.push(record);
        self.save(&records)
    }
    pub fn resolve(
        &self,
        signature: &str,
        state: &str,
        delta: Option<String>,
    ) -> Result<(), AppError> {
        let mut records = self
            .records
            .lock()
            .map_err(|_| AppError::cleanup("Journal unavailable"))?;
        if let Some(record) = records.iter_mut().find(|r| r.signature == signature) {
            record.state = state.into();
            record.delta = delta;
        }
        self.save(&records)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dock_flints_core::core::cleanup::CleanupOperation;
    fn path() -> PathBuf {
        std::env::temp_dir()
            .join(format!(
                "dock-journal-{}",
                solana_pubkey::Pubkey::new_unique()
            ))
            .join("signatures.json")
    }
    fn submission() -> PendingSubmission {
        PendingSubmission {
            wallet: "wallet".into(),
            account: "source".into(),
            mint: "mint".into(),
            operation: CleanupOperation::Close,
            signature: "1".repeat(64),
            expiry: 1000,
        }
    }
    #[test]
    fn restart_retains_pending_identity_without_secret_material_and_lock_is_exclusive() {
        let path = path();
        let journal = Journal::open(path.clone()).unwrap();
        let lease = journal.execution_lock().unwrap();
        journal.submit("job", "network", submission()).unwrap();
        let restarted = Journal::open(path.clone()).unwrap();
        assert_eq!(restarted.pending("network", "wallet").unwrap().len(), 1);
        assert!(
            restarted
                .pending("other-network", "wallet")
                .unwrap()
                .is_empty()
        );
        assert!(
            restarted
                .pending("network", "other-wallet")
                .unwrap()
                .is_empty()
        );
        assert!(restarted.execution_lock().is_err());
        assert!(restarted.submit("job", "network", submission()).is_err());
        assert_eq!(restarted.accounting("job").unwrap(), (0, false));
        let value = std::fs::read_to_string(&path).unwrap();
        for secret_field in ["seed", "keypair", "transaction", "base64"] {
            assert!(!value.contains(secret_field));
        }
        drop(lease);
        restarted.execution_lock().unwrap();
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }
    #[test]
    fn failed_persistence_rejects_submission_before_rpc_can_send() {
        let path = path();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::create_dir(path.with_extension("tmp")).unwrap();
        assert!(
            Journal::open(path.clone())
                .unwrap()
                .submit("job", "network", submission())
                .is_err()
        );
        assert!(!path.exists());
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }
    #[tokio::test]
    async fn unresolved_signature_stays_pending_then_confirmed_failure_fees_are_reconciled() {
        use serde_json::json;
        use solana_rpc_client_api::request::RpcRequest;
        let path = path();
        let journal = Journal::open(path.clone()).unwrap();
        journal.submit("job", "network", submission()).unwrap();
        let pending = solana_rpc_client::nonblocking::rpc_client::RpcClient::new_mock_with_mocks(
            "succeeds".into(),
            std::collections::HashMap::from([(
                RpcRequest::GetSignatureStatuses,
                json!({"context":{"slot":1},"value":[null]}),
            )]),
        );
        assert!(
            crate::cleanup::reconcile(&pending, &journal, "network", "wallet")
                .await
                .is_err()
        );
        assert_eq!(
            Journal::open(path.clone())
                .unwrap()
                .pending("network", "wallet")
                .unwrap()
                .len(),
            1
        );
        let confirmed = solana_rpc_client::nonblocking::rpc_client::RpcClient::new_mock_with_mocks(
            "succeeds".into(),
            std::collections::HashMap::from([
                (
                    RpcRequest::GetSignatureStatuses,
                    json!({"context":{"slot":1},"value":[{"slot":1,"confirmations":null,"err":{"InstructionError":[0,{"Custom":1}]},"status":{"Err":{"InstructionError":[0,{"Custom":1}]}},"confirmationStatus":"confirmed"}]}),
                ),
                (
                    RpcRequest::GetTransaction,
                    json!({"transaction":{"message":{"accountKeys":["wallet"]}},"meta":{"err":{"InstructionError":[0,{"Custom":1}]},"preBalances":[18446744073709551615u64],"postBalances":[18446744073709546615u64]}}),
                ),
            ]),
        );
        crate::cleanup::reconcile(&confirmed, &journal, "network", "wallet")
            .await
            .unwrap();
        let restarted = Journal::open(path.clone()).unwrap();
        assert!(restarted.pending("network", "wallet").unwrap().is_empty());
        assert_eq!(restarted.accounting("job").unwrap(), (-5000, true));
        assert!(
            restarted
                .submit("another-job", "network", submission())
                .is_err()
        );
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }
}
