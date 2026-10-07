fn main() {
    // Restrict the application command to explicitly granted local capabilities.
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "analyze_wallet",
            "connect_wallet",
            "disconnect_wallet",
        ]),
    ))
    .expect("failed to build the Tauri application manifest");
}
