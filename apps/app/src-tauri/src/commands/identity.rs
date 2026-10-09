use crate::{
    dto::identity::{ConnectWalletRequestDto, WalletConnectionDto},
    error::AppError,
    state::AppState,
};

/// Connect locally without making network calls or authorizing any transaction.
#[tauri::command]
pub async fn connect_wallet(
    state: tauri::State<'_, AppState>,
    request: serde_json::Value,
) -> Result<WalletConnectionDto, AppError> {
    // Deserialization diagnostics can contain supplied credentials, so never forward them.
    let request: ConnectWalletRequestDto = serde_json::from_value(request).map_err(|_| {
        AppError::invalid_identity("Choose exactly one valid wallet identity source")
    })?;
    let source_kind = request.source_kind();
    let identity = tauri::async_runtime::spawn_blocking(move || request.into_identity())
        .await
        .map_err(|_| AppError::invalid_identity("Local wallet loading could not be completed"))??;
    let mut store = state
        .store
        .lock()
        .map_err(|_| AppError::cleanup("Session unavailable"))?;
    if store.active_job.is_some() {
        return Err(AppError::cleanup(
            "Cleanup is active; wait for confirmation before changing wallet",
        ));
    }
    let session_id = crate::cleanup::new_id();
    let response = WalletConnectionDto {
        session_id: session_id.clone(),
        wallet_address: identity.address.to_string(),
        source_kind,
        can_sign: identity.signer().is_ok(),
    };
    store.plans.clear();
    store.revision = 0;
    store.session = Some(crate::cleanup::Session {
        id: session_id,
        identity: std::sync::Arc::new(identity),
    });
    Ok(response)
}

/// Disconnect cannot discard a signer while an approved operation is in flight.
#[tauri::command]
pub fn disconnect_wallet(state: tauri::State<'_, AppState>) -> Result<(), AppError> {
    let mut store = state
        .store
        .lock()
        .map_err(|_| AppError::cleanup("Session unavailable"))?;
    if store.active_job.is_some() {
        return Err(AppError::cleanup(
            "Cleanup is active; wait for confirmation before disconnecting",
        ));
    }
    store.session = None;
    store.plans.clear();
    Ok(())
}
