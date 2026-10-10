use crate::{cleanup::*, dto::cleanup::*, error::AppError, state::AppState};
use dock_flints_core::{
    app::{
        categories::ScopedSwap,
        cleanup::{
            execute::execute_plan_observed,
            plan::{add_nfts_observed, assets_from_portfolio, build_plan_observed},
        },
    },
    core::{ScanOptions, ScanSelection, ScanStatus, categories::now, cleanup::*, progress::*},
};
use std::sync::Arc;

/// Discover and store a read-only plan. IPC only carries selection, never transaction ingredients.
#[tauri::command]
pub async fn prepare_cleanup<R: tauri::Runtime>(
    webview: tauri::Webview<R>,
    progress: Option<tauri::ipc::JavaScriptChannelId>,
    state: tauri::State<'_, AppState>,
    request: serde_json::Value,
) -> Result<CleanupPlanDto, AppError> {
    let request: PrepareRequest = serde_json::from_value(request)
        .map_err(|_| AppError::cleanup("Invalid cleanup selection"))?;
    for mint in request
        .ignored_mints
        .iter()
        .chain(match &request.selection {
            SelectionDto::Selected { mints, .. } => mints.iter(),
            _ => request.ignored_mints.iter(),
        })
    {
        mint.parse::<solana_pubkey::Pubkey>()
            .map_err(|_| AppError::cleanup("Selection must use valid mint addresses"))?;
    }
    let identity = {
        let mut store = state
            .store
            .lock()
            .map_err(|_| AppError::cleanup("Session unavailable"))?;
        if store.active_job.is_some() {
            return Err(AppError::cleanup("Cleanup is already running"));
        }
        let session = store
            .session
            .as_ref()
            .filter(|s| s.id == request.session_id)
            .ok_or_else(|| AppError::cleanup("Wallet session changed"))?;
        let identity = session.identity.clone();
        if request.revision <= store.revision {
            return Err(AppError::cleanup("Stale selection"));
        }
        store.revision = request.revision;
        store.plans.clear();
        identity
    };
    let observer = ReadObserver {
        channel: progress.map(|id| id.channel_on(webview)),
        job_id: new_id(),
        session_id: request.session_id.clone(),
        sequence: Default::default(),
    };
    let (network, mainnet) = network(&state.rpc).await?;
    let snapshot = dock_flints_core::app::scan_wallet::scan_wallet_observed::<()>(
        &state.rpc,
        &identity.address,
        &ScanOptions {
            selection: ScanSelection {
                all_tokens: true,
                ..ScanSelection::ALL
            },
            no_prices: true,
        },
        None,
        &observer,
    )
    .await;
    let compressed = match &state.das {
        Some(das) => {
            das.compressed_on_network(&identity.address.to_string(), &network)
                .await
        }
        None => dock_flints_core::core::categories::CompressedReport {
            items: vec![],
            status: ScanStatus::Unsupported(
                "DAS unavailable; compressed inventory not checked".into(),
            ),
        },
    };
    let inventory = dock_flints_core::core::inventory::normalize(&snapshot, &compressed);
    let selected_inventory: Vec<_> = inventory
        .iter()
        .filter(|a| match &request.selection {
            SelectionDto::All => true,
            SelectionDto::None => false,
            SelectionDto::Selected { mints, asset_ids } => {
                asset_ids.contains(&a.id)
                    || a.mint
                        .as_ref()
                        .is_some_and(|mint| mints.contains(mint) || asset_ids.contains(mint))
            }
        })
        .collect();
    let mut ignored_mints = request.ignored_mints.clone();
    let mut ignored_asset_ids = request.ignored_asset_ids.clone();
    for asset in &inventory {
        if (request.ignored_asset_ids.contains(&asset.id)
            || asset
                .mint
                .as_ref()
                .is_some_and(|mint| request.ignored_asset_ids.contains(mint)))
            && let Some(mint) = &asset.mint
        {
            ignored_mints.insert(mint.clone());
            ignored_asset_ids.insert(mint.clone());
        }
    }
    let options = CleanupOptions {
        selection: CleanupSelection {
            selected_accounts_only: true,
            none: selected_inventory.is_empty(),
            accounts: selected_inventory
                .iter()
                .flat_map(|a| a.accounts.iter().cloned())
                .collect(),
            asset_ids: Some(
                selected_inventory
                    .iter()
                    .map(|a| a.mint.clone().unwrap_or(a.id.clone()))
                    .collect(),
            ),
            ignored_asset_ids,
            ignored_mints,
        },
        quote_interval: std::time::Duration::ZERO,
        ..Default::default()
    };
    let executor = dock_flints_core::infra::solana::nft_cleanup::SolanaCleanupExecutor {
        rpc: &state.rpc,
        das: state.das.as_ref(),
    };
    let assets = assets_from_portfolio(&snapshot);
    let provider = ScopedSwap {
        provider: state.jupiter.as_ref(),
        mainnet,
    };
    let mut plan = build_plan_observed(
        &identity.address,
        assets,
        snapshot
            .scanners
            .get("all_tokens")
            .cloned()
            .unwrap_or(ScanStatus::Failed("Discovery unavailable".into())),
        {
            snapshot
                .unknown_assets
                .iter()
                .filter(|asset| {
                    asset.lamports.is_some()
                        && !snapshot
                            .token_accounts
                            .iter()
                            .any(|known| known.address == asset.address)
                        && !snapshot
                            .core_assets
                            .iter()
                            .any(|known| known.address == asset.address)
                })
                .cloned()
                .collect()
        },
        &provider,
        &options,
        &observer,
    )
    .await;
    add_nfts_observed(
        &mut plan,
        &snapshot,
        &compressed,
        &executor,
        &options,
        &observer,
    )
    .await;
    let nft_burns = plan
        .nft_entries
        .iter()
        .filter(|e| e.prepared.is_some())
        .count();
    let plan_id = new_id();
    let expires = now() + 120;
    let dto = CleanupPlanDto {
        plan_id: plan_id.clone(),
        session_id: request.session_id.clone(),
        revision: request.revision,
        network: network.clone(),
        expires_at: expires.to_string(),
        entries: plan
            .entries
            .iter()
            .map(|entry| PlanEntryDto {
                asset_id: format!(
                    "{}:{}",
                    entry.asset.account.mint,
                    entry.asset.account.program.id()
                ),
                token_account: Some(entry.asset.account.address.clone()),
                account: entry.asset.account.address.clone(),
                mint: entry.asset.account.mint.clone(),
                program: entry.asset.account.program.id().to_string(),
                raw_amount: entry.asset.account.raw_amount.to_string(),
                action: match entry.category {
                    CleanupCategory::Empty => "close",
                    CleanupCategory::Swappable => "swap",
                    CleanupCategory::Burnable => "burn",
                    CleanupCategory::Unsupported => "skip",
                }
                .into(),
                reason: entry.reason.clone(),
                reason_code: entry.reason_code,
                decimals: entry.asset.account.decimals,
                supply: entry
                    .asset
                    .mint
                    .as_ref()
                    .map(|mint| mint.supply.to_string()),
                kind: format!("{:?}", entry.asset.kind),
                mint_authority: entry
                    .asset
                    .mint
                    .as_ref()
                    .and_then(|mint| mint.mint_authority.clone()),
                freeze_authority: entry
                    .asset
                    .mint
                    .as_ref()
                    .and_then(|mint| mint.freeze_authority.clone()),
                close_authority: entry.asset.account.close_authority.clone(),
                account_extensions: entry.asset.account.extension_types.clone(),
                mint_extensions: entry
                    .asset
                    .mint
                    .as_ref()
                    .map(|mint| mint.extension_types.clone())
                    .unwrap_or_default(),
                expected_out_lamports: entry
                    .quote
                    .as_ref()
                    .map(|q| q.expected_out_lamports.to_string()),
                min_out_lamports: entry.quote.as_ref().map(|q| q.min_out_lamports.to_string()),
            })
            .chain(plan.nft_entries.iter().map(|entry| {
                PlanEntryDto {
                    asset_id: entry
                        .target
                        .mint
                        .as_ref()
                        .map(|mint| {
                            format!(
                                "{}:{}",
                                mint,
                                dock_flints_core::core::TokenProgram::Legacy.id()
                            )
                        })
                        .unwrap_or(entry.target.id.clone()),
                    token_account: entry.target.token_account.clone(),
                    account: entry
                        .target
                        .token_account
                        .clone()
                        .unwrap_or(entry.target.id.clone()),
                    mint: entry.target.mint.clone().unwrap_or_default(),
                    program: match entry.target.standard {
                        dock_flints_core::core::nft_cleanup::NftStandard::TokenMetadata(_) => {
                            "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s"
                        }
                        dock_flints_core::core::nft_cleanup::NftStandard::Core => {
                            "CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d"
                        }
                        dock_flints_core::core::nft_cleanup::NftStandard::Compressed => {
                            "BGUMAp9Gq7iTEuizy4pqaxsTyUCBK68MDfK752saRPUY"
                        }
                    }
                    .into(),
                    raw_amount: "1".into(),
                    action: if entry.prepared.is_some() {
                        "burn"
                    } else {
                        "skip"
                    }
                    .into(),
                    reason: entry.reason.clone(),
                    reason_code: entry.reason_code,
                    decimals: None,
                    supply: None,
                    kind: format!("{:?}", entry.target.standard),
                    mint_authority: None,
                    freeze_authority: None,
                    close_authority: None,
                    account_extensions: vec![],
                    mint_extensions: vec![],
                    expected_out_lamports: None,
                    min_out_lamports: None,
                }
            }))
            .collect(),
        selected_assets: selected_inventory
            .iter()
            .filter(|a| {
                !options.selection.ignored_asset_ids.contains(&a.id)
                    && !a
                        .mint
                        .as_ref()
                        .is_some_and(|mint| options.selection.ignored_mints.contains(mint))
            })
            .count(),
        executable_accounts: plan.summary.accounts_to_close + nft_burns,
        skipped_accounts: plan.summary.unsupported,
        undecodable_accounts: plan
            .unparsed_accounts
            .iter()
            .map(|account| UndecodableAccountDto {
                address: account.address.clone(),
                program: account.program_id.clone(),
                lamports: account.lamports.map(|value| value.to_string()),
                reason_code: CleanupReasonCode::Undecodable,
                reason: account.reason.clone(),
            })
            .collect(),
        can_execute: identity.signer().is_ok() && (plan.summary.accounts_to_close + nft_burns) > 0,
        requires_burn: (plan.summary.burnable + nft_burns) > 0,
        swap_count: plan.summary.swappable,
        burn_count: plan.summary.burnable + nft_burns,
        close_count: plan.summary.accounts_to_close,
        estimated_swap_lamports: plan.summary.estimated_swap_lamports.to_string(),
        estimated_reclaimed_lamports: plan.summary.estimated_reclaimed_lamports.to_string(),
        max_price_impact_bps: options.swap_limits.max_price_impact_bps,
        max_priority_fee_lamports: options.swap_limits.max_priority_fee_lamports.to_string(),
        slippage_bps: options.slippage_bps,
    };
    let mut store = state
        .store
        .lock()
        .map_err(|_| AppError::cleanup("Session unavailable"))?;
    if store
        .session
        .as_ref()
        .is_none_or(|s| s.id != request.session_id)
        || store.revision != request.revision
    {
        return Err(AppError::cleanup(
            "Selection/session changed while planning",
        ));
    }
    store.plans.insert(
        plan_id,
        StoredPlan {
            session_id: request.session_id,
            revision: request.revision,
            network,
            expires,
            core: Arc::new(plan),
            options,
            dto: dto.clone(),
            job_id: None,
        },
    );
    Ok(dto)
}

/// Claim an approved server plan exactly once; subsequent IPC requests return the same job.
#[tauri::command]
pub async fn execute_cleanup<R: tauri::Runtime>(
    webview: tauri::Webview<R>,
    state: tauri::State<'_, AppState>,
    request: serde_json::Value,
    progress: Option<tauri::ipc::JavaScriptChannelId>,
) -> Result<CleanupJobDto, AppError> {
    let request: ExecuteRequest = serde_json::from_value(request)
        .map_err(|_| AppError::cleanup("Invalid cleanup approval"))?;
    let (network, mainnet) = network(&state.rpc).await?;
    let owner = {
        let store = state
            .store
            .lock()
            .map_err(|_| AppError::cleanup("Session unavailable"))?;
        let session = store
            .session
            .as_ref()
            .filter(|s| s.id == request.session_id)
            .ok_or_else(|| AppError::cleanup("Wallet session changed"))?;
        session.identity.address.to_string()
    };
    {
        let store = state
            .store
            .lock()
            .map_err(|_| AppError::cleanup("Session unavailable"))?;
        if let Some(plan) = store.plans.get(&request.plan_id)
            && plan.session_id == request.session_id
            && plan.network == network
            && let Some(id) = &plan.job_id
        {
            return store
                .jobs
                .get(id)
                .cloned()
                .ok_or_else(|| AppError::cleanup("Job unavailable"));
        }
    }
    let _execution_lease = state.journal.execution_lock()?;
    reconcile(&state.rpc, &state.journal, &network, &owner).await?;
    let (id, identity, plan, options, run) = {
        let mut store = state
            .store
            .lock()
            .map_err(|_| AppError::cleanup("Session unavailable"))?;
        let existing = store
            .plans
            .get(&request.plan_id)
            .and_then(|p| p.job_id.as_ref())
            .is_some();
        let (id, identity, plan, options) = store.approve(&request, &network)?;
        (id, identity, plan, options, !existing)
    };
    if run {
        let observer = JobObserver {
            store: state.store.clone(),
            journal: state.journal.clone(),
            channel: progress.map(|id| id.channel_on(webview)),
            job_id: id.clone(),
            session_id: request.session_id.clone(),
            network: network.clone(),
        };
        let provider = ScopedSwap {
            provider: state.jupiter.as_ref(),
            mainnet,
        };
        let signer = identity
            .signer()
            .map_err(|_| AppError::cleanup("Signer unavailable"))?;
        let result = execute_plan_observed(
            &plan,
            &provider,
            &dock_flints_core::infra::solana::nft_cleanup::SolanaCleanupExecutor {
                rpc: &state.rpc,
                das: state.das.as_ref(),
            },
            signer,
            &options,
            &observer,
        )
        .await;
        // Receipts are already confirmed; journal metadata can be reconciled without resending.
        observer.progress(CleanupProgress {
            stage: "accounting".into(),
            completed: plan.entries.len() + plan.nft_entries.len(),
            total: plan.entries.len() + plan.nft_entries.len(),
            operation: None,
            account: None,
            status: "running".into(),
        });
        let reconciliation = reconcile(&state.rpc, &state.journal, &network, &owner).await;
        let accounting = state.journal.accounting(&id);
        let mut store = state
            .store
            .lock()
            .map_err(|_| AppError::cleanup("Session unavailable"))?;
        let job = store
            .jobs
            .get_mut(&id)
            .ok_or_else(|| AppError::cleanup("Job unavailable"))?;
        match result {
            Ok(mut report) => {
                match accounting {
                    Ok((delta, complete)) => {
                        report.known_net_wallet_lamports = delta;
                        report.accounting_complete &= complete && reconciliation.is_ok();
                    }
                    Err(_) => report.accounting_complete = false,
                }
                job.status = if report.failed == 0
                    && report.accounting_complete
                    && !plan.has_blocked_selection()
                {
                    "completed"
                } else if report.completed > 0 {
                    "partial"
                } else {
                    "failed"
                }
                .into();
                job.report = Some(report);
                job.error = reconciliation.err().map(|e| e.message);
            }
            Err(error) => {
                job.status = "failed".into();
                job.error = Some(error.to_string());
            }
        }
        let terminal = job.status.clone();
        store.active_job = None;
        drop(store);
        observer.progress(CleanupProgress {
            stage: terminal,
            completed: plan.entries.len() + plan.nft_entries.len(),
            total: plan.entries.len() + plan.nft_entries.len(),
            operation: None,
            account: None,
            status: "complete".into(),
        });
    }
    let store = state
        .store
        .lock()
        .map_err(|_| AppError::cleanup("Session unavailable"))?;
    store
        .jobs
        .get(&id)
        .cloned()
        .ok_or_else(|| AppError::cleanup("Job unavailable"))
}

/// Reattach to a job even if its progress channel was closed. Another session cannot read it.
#[tauri::command]
pub fn get_cleanup_job(
    state: tauri::State<'_, AppState>,
    request: serde_json::Value,
) -> Result<CleanupJobDto, AppError> {
    let request: JobRequest =
        serde_json::from_value(request).map_err(|_| AppError::cleanup("Invalid job request"))?;
    let store = state
        .store
        .lock()
        .map_err(|_| AppError::cleanup("Session unavailable"))?;
    if store
        .session
        .as_ref()
        .is_none_or(|s| s.id != request.session_id)
    {
        return Err(AppError::cleanup("Wallet session changed"));
    }
    let job_id = match (&request.job_id, &request.plan_id) {
        (Some(id), None) => id,
        (None, Some(plan_id)) => store
            .plans
            .get(plan_id)
            .filter(|p| p.session_id == request.session_id)
            .and_then(|p| p.job_id.as_ref())
            .ok_or_else(|| AppError::cleanup("No execution was claimed for this plan"))?,
        _ => return Err(AppError::cleanup("Provide exactly one jobId or planId")),
    };
    store
        .jobs
        .get(job_id)
        .filter(|j| j.session_id == request.session_id)
        .cloned()
        .ok_or_else(|| AppError::cleanup("Job unavailable"))
}
