//! Desktop identity, inspection and approved cleanup adapter. Business logic remains in dock-flints-core.
pub mod cleanup;
pub mod commands;
pub mod dto;
pub mod error;
pub mod journal;
pub mod state;

use tauri::Manager;

/// Initialize shared clients and register local identity, inspection and approved cleanup commands.
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            app.manage(state::AppState::from_env()?);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::wallet::analyze_wallet,
            commands::identity::connect_wallet,
            commands::identity::disconnect_wallet,
            commands::cleanup::prepare_cleanup,
            commands::cleanup::execute_cleanup,
            commands::cleanup::get_cleanup_job
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Dock Flints desktop backend");
}
