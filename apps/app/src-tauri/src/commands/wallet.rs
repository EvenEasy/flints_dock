use crate::{
    dto::wallet::{AnalyzeWalletRequestDto, WalletAnalysisDto},
    error::AppError,
    state::AppState,
};
use dock_flints_core::{app::scan_wallet::scan_wallet_observed, core::ScanStatus};

/// Analyze selected wallet categories through the existing read-only core use case.
/// Invalid addresses reject the request; partial RPC/pricing failures remain in the response.
#[tauri::command]
pub async fn analyze_wallet<R: tauri::Runtime>(
    webview: tauri::Webview<R>,
    progress: Option<tauri::ipc::JavaScriptChannelId>,
    state: tauri::State<'_, AppState>,
    request: serde_json::Value,
) -> Result<WalletAnalysisDto, AppError> {
    // Parse inside the command so malformed DTO fields use the same stable error envelope.
    let request: AnalyzeWalletRequestDto = serde_json::from_value(request)
        .map_err(|error| AppError::invalid_request(&error.to_string()))?;

    // Validate public identity and translate category selection before any network work.
    let (owner, options) = request.into_core()?;
    use dock_flints_core::core::progress::CleanupObserver;
    let session_id = state
        .store
        .lock()
        .ok()
        .and_then(|s| {
            s.session
                .as_ref()
                .filter(|s| s.identity.address == owner)
                .map(|s| s.id.clone())
        })
        .unwrap_or_default();
    let observer = crate::cleanup::ReadObserver {
        channel: progress.map(|id| id.channel_on(webview)),
        job_id: crate::cleanup::new_id(),
        session_id,
        sequence: Default::default(),
    };
    let scope = crate::cleanup::network(&state.rpc).await.ok();
    let mainnet = scope.as_ref().is_some_and(|(_, mainnet)| *mainnet);
    let mut snapshot = scan_wallet_observed(
        &state.rpc,
        &owner,
        &options,
        if mainnet {
            state.jupiter.as_ref()
        } else {
            None
        },
        &observer,
    )
    .await;
    // Preserve the CLI's pricing initialization failure without hiding successful holdings.
    if options.selection.needs_prices()
        && !options.no_prices
        && let Some(reason) = &state.pricing_error
    {
        snapshot
            .scanners
            .insert("prices".into(), ScanStatus::Failed(reason.clone()));
    }

    if !mainnet && options.selection.needs_prices() && !options.no_prices {
        snapshot.scanners.insert(
            "prices".into(),
            ScanStatus::Unsupported(
                "Jupiter valuation is mainnet-only; network is different or could not be verified"
                    .into(),
            ),
        );
    }
    observer.progress(dock_flints_core::core::progress::CleanupProgress {
        stage: "risk/routing/DAS".into(),
        completed: 3,
        total: 4,
        operation: None,
        account: None,
        status: "running".into(),
    });
    let mut compressed_inventory = None;
    let mut normalized_inventory = None;
    let categories = if options.selection.needs_token_accounts()
        || options.selection.nfts
        || options.selection.cnfts
    {
        let compressed = match &state.das {
            Some(das) => {
                das.compressed_on_network(
                    &owner.to_string(),
                    scope
                        .as_ref()
                        .map(|(hash, _)| hash.as_str())
                        .unwrap_or("unknown"),
                )
                .await
            }
            None => dock_flints_core::core::categories::CompressedReport {
                items: vec![],
                status: ScanStatus::Unsupported(
                    "DAS endpoint not configured; compressed NFT inventory unavailable".into(),
                ),
            },
        };
        if options.selection.cnfts {
            compressed_inventory = Some(crate::dto::assets::CompressedNftsDto {
                status: compressed.status.clone().into(),
                items: matches!(
                    compressed.status,
                    ScanStatus::Complete | ScanStatus::Partial(_)
                )
                .then(|| compressed.items.clone()),
            });
        }
        let items = dock_flints_core::core::inventory::normalize(&snapshot, &compressed);
        let onchain = snapshot
            .scanners
            .get("all_tokens")
            .or(snapshot.scanners.get("tokens"))
            .cloned()
            .unwrap_or(ScanStatus::Skipped("Token inventory not requested".into()));
        let coverage = dock_flints_core::core::combine_statuses(&[
            ("tokens", &onchain),
            (
                "Core",
                snapshot
                    .scanners
                    .get("core_asset_v1")
                    .unwrap_or(&ScanStatus::Skipped("Core not requested".into())),
            ),
            ("compressed", &compressed.status),
        ]);
        normalized_inventory = Some(crate::dto::assets::AssetListDto {
            status: coverage.clone().into(),
            items: (!items.is_empty()
                || matches!(coverage, ScanStatus::Complete | ScanStatus::Partial(_)))
            .then_some(items),
        });
        let category_report = dock_flints_core::app::categories::classify(
            &snapshot,
            &owner,
            state.jupiter.as_ref(),
            scope
                .map(|(hash, _)| hash)
                .unwrap_or_else(|| "unknown".into()),
            mainnet,
            state.dust_threshold_usd,
            compressed,
        )
        .await;
        Some(category_report.into())
    } else {
        None
    };

    let mut response: WalletAnalysisDto = snapshot.into();
    response.categories = categories;
    response.inventory = normalized_inventory;
    if let Some(inventory) = compressed_inventory {
        response
            .scanners
            .insert("compressed_nfts".into(), inventory.status.clone());
        response.has_usable_results |= inventory.items.is_some();
        response.cnfts = Some(inventory);
    }
    observer.progress(dock_flints_core::core::progress::CleanupProgress {
        stage: "completed".into(),
        completed: 4,
        total: 4,
        operation: None,
        account: None,
        status: "complete".into(),
    });
    Ok(response)
}
