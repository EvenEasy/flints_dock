//! Durable nonsecret submission records for owned local-validator examples.
use dock_flints_core::core::progress::*;
pub struct Journal(pub std::path::PathBuf);
impl CleanupObserver for Journal {
    fn before_send(
        &self,
        submission: PendingSubmission,
    ) -> Result<(), dock_flints_core::core::error::SwapError> {
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(&self.0)
            .map_err(|e| dock_flints_core::core::error::SwapError::InvalidRequest(e.to_string()))?;
        writeln!(file, "{}", serde_json::to_string(&submission).unwrap())
            .map_err(|e| dock_flints_core::core::error::SwapError::InvalidRequest(e.to_string()))?;
        file.sync_all()
            .map_err(|e| dock_flints_core::core::error::SwapError::InvalidRequest(e.to_string()))
    }
}
