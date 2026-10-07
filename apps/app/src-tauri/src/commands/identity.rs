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
    let response = WalletConnectionDto {
        wallet_address: identity.address.to_string(),
        source_kind,
        can_sign: identity.signer().is_ok(),
    };

    // Replacing the session drops the previous signer; only the backend retains private material.
    *state
        .identity
        .lock()
        .map_err(|_| AppError::configuration("walletSession", "Wallet session is unavailable"))? =
        Some(identity);
    Ok(response)
}

/// Forget the local signer; no files, balances, or blockchain accounts are changed.
#[tauri::command]
pub fn disconnect_wallet(state: tauri::State<'_, AppState>) -> Result<(), AppError> {
    *state
        .identity
        .lock()
        .map_err(|_| AppError::configuration("walletSession", "Wallet session is unavailable"))? =
        None;
    Ok(())
}
