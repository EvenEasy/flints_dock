use super::{plan::*, *};
use crate::app::swap::{SwapError, SwapProvider};
use solana_signer::Signer;

fn changed(message: &str) -> SwapError {
    SwapError::InvalidRequest(message.into())
}
fn same_account(expected: &CleanupAsset, actual: &CleanupAsset) -> bool {
    expected.account.address == actual.account.address
        && expected.account.mint == actual.account.mint
        && expected.account.program == actual.account.program
        && expected.account.owner == actual.account.owner
        && expected.account.decimals == actual.account.decimals
}
async fn run_entry(
    entry: &CleanupEntry,
    result: &mut AccountCleanupResult,
    owner: &Pubkey,
    signer: &Keypair,
    provider: &impl SwapProvider,
    executor: &impl CleanupExecutor,
    options: &CleanupOptions,
) -> Result<()> {
    let current = executor
        .refresh(&entry.asset.account.address, owner)
        .await?
        .ok_or_else(|| changed("account already absent; no action"))?;

    // Reject identity or balance changes instead of silently expanding the approved operation.
    if !same_account(&entry.asset, &current)
        || current.account.raw_amount != entry.asset.account.raw_amount
    {
        return Err(changed(
            "account identity or balance changed since preview; rebuild the plan",
        ));
    }
    if let Some(reason) = unsupported_reason(&current, &owner.to_string()) {
        return Err(changed(&reason));
    }

    // Follow the approved category without converting a failed swap into a burn.
    match entry.category {
        CleanupCategory::Swappable => {
            let request = request(&current, owner, options)?;
            let approved_min = entry
                .quote
                .as_ref()
                .ok_or_else(|| changed("missing approved quote"))?
                .min_out_lamports;
            let mut attempt = 0;
            loop {
                let fresh = fresh_route(provider, &request, options).await?;
                if fresh.quote.min_out_lamports < approved_min {
                    return Err(SwapError::PreviewChanged);
                }
                match executor
                    .perform(
                        CleanupOperation::Swap,
                        &current,
                        Some(fresh),
                        signer,
                        &options.swap_limits,
                    )
                    .await
                {
                    Ok(receipt) => {
                        result.operations.push(receipt);
                        break;
                    }

                    // These failures occur before send. Never retry an uncertain submission.
                    Err(SwapError::Expired | SwapError::Simulation(_)) if attempt == 0 => {
                        attempt += 1;
                    }
                    Err(error) => return Err(error),
                }
            }
        }
        CleanupCategory::Burnable => {
            // A route may have appeared during a long plan/approval interval.
            // Only a second explicit NoRoute permits the already-previewed burn.
            match fresh_route(provider, &request(&current, owner, options)?, options).await {
                Err(SwapError::NoRoute(_)) => {}
                Ok(_) => {
                    return Err(changed(
                        "a swap route now exists; burn skipped, rebuild plan",
                    ));
                }
                Err(error) => return Err(error),
            }
            result.operations.push(
                executor
                    .perform(
                        CleanupOperation::Burn,
                        &current,
                        None,
                        signer,
                        &options.swap_limits,
                    )
                    .await?,
            );
        }
        CleanupCategory::Empty => {}
        CleanupCategory::Unsupported => return Err(changed("unsupported plan entry")),
    }

    // Never assume a swap or burn consumed the particular account's entire balance.
    let empty = executor
        .refresh(&entry.asset.account.address, owner)
        .await?
        .ok_or_else(|| {
            changed("source disappeared before explicit close; inspect confirmed operations")
        })?;
    if !same_account(&entry.asset, &empty) || empty.account.raw_amount != 0 {
        return Err(changed(
            "post-operation source balance is not zero or identity changed; not closed",
        ));
    }
    if let Some(reason) = unsupported_reason(&empty, &owner.to_string()) {
        return Err(changed(&reason));
    }
    result.operations.push(
        executor
            .perform(
                CleanupOperation::Close,
                &empty,
                None,
                signer,
                &options.swap_limits,
            )
            .await?,
    );
    if executor
        .refresh(&entry.asset.account.address, owner)
        .await?
        .is_some()
    {
        return Err(changed(
            "close confirmed but account still exists on recheck",
        ));
    }
    Ok(())
}

/// Return per-account outcomes after sequentially executing the approved plan.
/// Rejects a mismatched signer; individual failures retain receipts and do not stop unrelated
/// mints.
/// Uncertain submissions block further actions for that mint, without automatic resubmission.
pub async fn execute_plan(
    plan: &CleanupPlan,
    provider: &impl SwapProvider,
    executor: &impl CleanupExecutor,
    signer: &Keypair,
    options: &CleanupOptions,
) -> Result<CleanupReport> {
    let owner = signer.pubkey();
    if owner.to_string() != plan.wallet {
        return Err(changed("keypair does not match cleanup wallet"));
    }
    let mut report = CleanupReport {
        accounting_complete: true,
        ..Default::default()
    };

    // One account and one transaction at a time. Partial successes are retained.
    let mut unresolved_mints = std::collections::BTreeSet::new();
    for entry in &plan.entries {
        let mut result = AccountCleanupResult {
            token_account: entry.asset.account.address.clone(),
            mint: entry.asset.account.mint.clone(),
            category: entry.category,
            status: "skipped".into(),
            reason: entry.reason.clone(),
            operations: vec![],
            uncertain_signature: None,
        };

        // Enforce both saved approval and current restrictions before any RPC mutation.
        let protected = plan
            .selection
            .skip_reason(&entry.asset.account)
            .or_else(|| options.selection.skip_reason(&entry.asset.account))
            .or_else(|| {
                (entry.category == CleanupCategory::Swappable
                    && (plan.selection.protects_output() || options.selection.protects_output()))
                .then_some("protected WSOL output account; swap skipped")
            });
        if let Some(reason) = protected {
            result.reason = reason.into();
            report.skipped += 1;
            report.results.push(result);
            continue;
        }

        // Avoid another economic action for a mint whose previous submission is unresolved.
        if unresolved_mints.contains(&entry.asset.account.mint) {
            result.reason = "previous transaction for this mint is unresolved; skipped".into();
            report.skipped += 1;
            report.results.push(result);
            continue;
        }
        if entry.category != CleanupCategory::Unsupported {
            match run_entry(
                entry,
                &mut result,
                &owner,
                signer,
                provider,
                executor,
                options,
            )
            .await
            {
                Ok(()) => {
                    result.status = "closed".into();
                    result.reason = "confirmed and account absence verified".into();
                    report.closed += 1;
                }
                Err(error) => {
                    result.status = "failed".into();
                    if let SwapError::Uncertain { signature, .. } = &error {
                        unresolved_mints.insert(entry.asset.account.mint.clone());
                        result.status = "uncertain".into();
                        result.uncertain_signature = Some(signature.clone());
                        report.accounting_complete = false;
                    }
                    if matches!(error, SwapError::TransactionFailed { .. }) {
                        report.accounting_complete = false;
                    }
                    result.reason = error.to_string();
                    report.failed += 1;
                }
            }
        } else {
            report.skipped += 1;
        }

        // Sum observed receipts only; estimates never replace missing transaction accounting.
        for receipt in &result.operations {
            if receipt.operation == CleanupOperation::Swap {
                if let Some(delta) = receipt.wallet_delta_lamports {
                    report.known_swap_net_lamports += delta;
                } else {
                    report.accounting_complete = false;
                }
            }
            if receipt.operation == CleanupOperation::Close {
                if let Some(lamports) = receipt.reclaimed_lamports {
                    report.known_reclaimed_lamports += u128::from(lamports);
                } else {
                    report.accounting_complete = false;
                }
            }
        }
        report.results.push(result);
    }

    // Include undecodable accounts in the final skipped list rather than hiding them.
    for unknown in &plan.unparsed_accounts {
        report.skipped += 1;
        report.results.push(AccountCleanupResult {
            token_account: unknown.address.clone(),
            mint: "unknown".into(),
            category: CleanupCategory::Unsupported,
            status: "skipped".into(),
            reason: unknown.reason.clone(),
            operations: vec![],
            uncertain_signature: None,
        });
    }
    Ok(report)
}
