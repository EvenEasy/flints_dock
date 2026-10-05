use crate::{
    dto::wallet::{AnalyzeWalletRequestDto, WalletAnalysisDto},
    error::AppError,
    state::AppState,
};
use dock_flints_core::{app::scan_wallet::scan_wallet, core::ScanStatus};

/// Analyze selected wallet categories through the existing read-only core use case.
/// Invalid addresses reject the request; partial RPC/pricing failures remain in the response.
#[tauri::command]
pub async fn analyze_wallet(
    state: tauri::State<'_, AppState>,
    request: serde_json::Value,
) -> Result<WalletAnalysisDto, AppError> {
    // Parse inside the command so malformed DTO fields use the same stable error envelope.
    let request: AnalyzeWalletRequestDto = serde_json::from_value(request)
        .map_err(|error| AppError::invalid_request(&error.to_string()))?;

    // Validate public identity and translate category selection before any network work.
    let (owner, options) = request.into_core()?;
    let mut snapshot = scan_wallet(&state.rpc, &owner, &options, state.jupiter.as_ref()).await;

    // Preserve the CLI's pricing initialization failure without hiding successful holdings.
    if options.selection.needs_prices()
        && !options.no_prices
        && let Some(reason) = &state.pricing_error
    {
        snapshot
            .scanners
            .insert("prices".into(), ScanStatus::Failed(reason.clone()));
    }

    Ok(snapshot.into())
}
