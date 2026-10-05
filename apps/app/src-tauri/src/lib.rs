//! Read-only desktop adapter. Scanning and classification remain in dock-flints-core.
pub mod commands;
pub mod dto;
pub mod error;
pub mod state;

use tauri::Manager;

/// Initialize shared clients once and register the read-only IPC surface.
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            app.manage(state::AppState::from_env()?);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![commands::wallet::analyze_wallet])
        .run(tauri::generate_context!())
        .expect("failed to run Dock Flints desktop backend");
}
