use crate::{cleanup::*, dto::cleanup::*, error::AppError, state::AppState};
use dock_flints_core::{
    app::{
        categories::ScopedSwap,
        cleanup::{
            execute::execute_plan_observed,
            plan::{assets_from_portfolio, build_plan_observed},
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
            SelectionDto::Selected { mints } => mints.iter(),
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
                ..Default::default()
            },
            no_prices: true,
        },
        None,
        &observer,
    )
    .await;
    let assets = assets_from_portfolio(&snapshot);
    // Expand each selected mint to ALL its accounts, including zero-balance accounts.
    let selected: Vec<_> = assets
        .into_iter()
        .filter(|a| match &request.selection {
            SelectionDto::All => true,
            SelectionDto::None => false,
            SelectionDto::Selected { mints } => mints.contains(&a.account.mint),
        })
        .collect();
    let options = CleanupOptions {
        selection: CleanupSelection {
            accounts: selected.iter().map(|a| a.account.address.clone()).collect(),
            ignored_mints: request.ignored_mints.clone(),
        },
        quote_interval: std::time::Duration::ZERO,
        ..Default::default()
    };
    let provider = ScopedSwap {
        provider: state.jupiter.as_ref(),
        mainnet,
    };
    let plan = build_plan_observed(
        &identity.address,
        selected,
        snapshot
            .scanners
            .get("all_tokens")
            .cloned()
            .unwrap_or(ScanStatus::Failed("Discovery unavailable".into())),
        if matches!(request.selection, SelectionDto::All) {
            snapshot
                .unknown_assets
                .iter()
                .filter(|asset| {
                    asset.lamports.is_some()
                        && asset.program_id.as_ref().is_some_and(|id| {
                            [
                                dock_flints_core::core::TokenProgram::Legacy,
                                dock_flints_core::core::TokenProgram::Token2022,
                            ]
                            .iter()
                            .any(|program| program.id().to_string() == *id)
                        })
                })
                .cloned()
                .collect()
        } else {
            vec![]
        },
        &provider,
        &options,
        &observer,
    )
    .await;
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
                expected_out_lamports: entry
                    .quote
                    .as_ref()
                    .map(|q| q.expected_out_lamports.to_string()),
                min_out_lamports: entry.quote.as_ref().map(|q| q.min_out_lamports.to_string()),
            })
            .collect(),
        can_execute: identity.signer().is_ok() && plan.summary.accounts_to_close > 0,
        requires_burn: plan.summary.burnable > 0,
        swap_count: plan.summary.swappable,
        burn_count: plan.summary.burnable,
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
        let result =
            execute_plan_observed(&plan, &provider, &state.rpc, signer, &options, &observer).await;
        // Receipts are already confirmed; journal metadata can be reconciled without resending.
        observer.progress(CleanupProgress {
            stage: "accounting".into(),
            completed: plan.entries.len(),
            total: plan.entries.len(),
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
                job.status = if report.failed == 0 && report.accounting_complete {
                    "completed"
                } else if report.closed > 0 {
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
            completed: plan.entries.len(),
            total: plan.entries.len(),
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
