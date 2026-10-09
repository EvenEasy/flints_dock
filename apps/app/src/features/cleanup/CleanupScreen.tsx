import type { CleanupPlan, CleanupProgress } from '../../../frontend-contract/cleanup';
import { lamportsToSol } from '../../shared/format';
import type { WalletAnalysis } from '../../../frontend-contract/types';
import { Brand, MascotHero, MechanicalPanel } from '../../shared/ui/Design';
import { RecoverButton } from '../../shared/ui/ActionButton';
import { AssetRow } from '../assets/AssetRow';
import { selectableAssets } from '../assets/presentation';
import { previewAssets } from '../preview/designData';
import { CategoryNotice, Notice } from '../../shared/ui/Notice';

/** Mint intent is expanded into account-scoped actions by the backend; estimates are never receipts. */
export function CleanupScreen({
  preview,
  analysis,
  ignoredMints,
  onToggleMint,
  onPreviewComplete,
  plan,
  preparing = false,
  planningProgress,
  error,
  canSign = false,
  onExecute,
  onRetry,
}: {
  preview: boolean;
  analysis: WalletAnalysis | null;
  ignoredMints: ReadonlySet<string>;
  onToggleMint: (mint: string) => void;
  onPreviewComplete: () => void;
  plan?: CleanupPlan | null;
  preparing?: boolean;
  planningProgress?: CleanupProgress | null;
  error?: string | null;
  canSign?: boolean;
  onExecute?: () => void;
  onRetry?: () => void;
}) {
  const assets = selectableAssets(analysis);
  const rows = preview ? previewAssets : assets;
  const selectedCount = rows.filter((row) => !ignoredMints.has(row.key)).length;
  const hasInventory =
    analysis?.tokens?.items != null ||
    analysis?.allTokens?.items != null ||
    analysis?.nfts?.classic.items != null;
  return (
    <div className="page page--cleanup">
      <Brand />
      <MascotHero variant="corner" />
      <header className="cleanup-header">
        <h1 className="type-screen" id="screen-heading" tabIndex={-1}>
          CLEANUP
        </h1>
        <p>Uncheck the assets you want to keep.</p>
      </header>
      <MechanicalPanel className="asset-manifest">
        <h2 className="manifest-columns">
          <span>ASSET</span>
          <span>VALUE IN SOL</span>
        </h2>
        {!preview && (
          <CategoryNotice status={analysis?.tokens?.status ?? analysis?.allTokens?.status} />
        )}
        <ul className="asset-list" aria-label="Assets to clean up">
          {preview
            ? previewAssets.map((asset) => (
                <AssetRow
                  key={asset.key}
                  identity={asset.key}
                  name={asset.name}
                  quantity={asset.quantity}
                  value={asset.value}
                  art={asset.art}
                  dead={asset.dead}
                  selected={!ignoredMints.has(asset.key)}
                  onToggle={() => onToggleMint(asset.key)}
                />
              ))
            : assets.map((asset) => (
                <AssetRow
                  key={asset.key}
                  identity={asset.mint}
                  name={asset.name}
                  quantity={asset.quantity}
                  value={
                    plan
                      ? (() => {
                          const entries = plan.entries.filter((entry) => entry.mint === asset.mint);
                          const estimate = entries.reduce(
                            (total, entry) => total + BigInt(entry.expectedOutLamports ?? '0'),
                            0n,
                          );
                          return estimate > 0n
                            ? `${lamportsToSol(estimate.toString())} SOL`
                            : entries.some((entry) => entry.action === 'burn')
                              ? 'BURN'
                              : entries.some((entry) => entry.action === 'close')
                                ? 'CLOSE'
                                : 'SKIP';
                        })()
                      : preparing
                        ? 'PLANNING'
                        : 'NOT ESTIMATED'
                  }
                  selected={!ignoredMints.has(asset.mint)}
                  onToggle={() => onToggleMint(asset.mint)}
                />
              ))}
        </ul>
        {!preview && rows.length === 0 && (
          <p className="empty-state">
            {hasInventory
              ? 'No token assets found in this category.'
              : 'Token inventory is unavailable. This is not an empty wallet.'}
          </p>
        )}
      </MechanicalPanel>
      <MechanicalPanel className="selection-summary">
        <p className="selection-count">
          SELECTED:{' '}
          <strong>
            {selectedCount} {selectedCount === 1 ? 'ASSET' : 'ASSETS'}
          </strong>
        </p>
        <div className="selection-estimate">
          <p>ESTIMATED RETURN</p>
          <strong>
            {preview
              ? selectedCount === 0
                ? '—'
                : '≈ 0.428 SOL'
              : plan
                ? `≈ ${lamportsToSol((BigInt(plan.estimatedSwapLamports) + BigInt(plan.estimatedReclaimedLamports)).toString())} SOL`
                : 'NOT ESTIMATED'}
          </strong>
        </div>
      </MechanicalPanel>
      {!preview && (
        <div className="cleanup-status type-caption" role="status">
          {error ? (
            <Notice tone="red" alert>
              {error}
              <button className="text-button" type="button" onClick={onRetry}>
                REFRESH PLAN
              </button>
            </Notice>
          ) : preparing ? (
            planningProgress ? (
              `${planningProgress.completed} / ${planningProgress.total} · ${planningProgress.stage === 'planning' ? 'Planning' : 'Checking accounts'}`
            ) : (
              'Checking routes and accounts…'
            )
          ) : !canSign ? (
            'Read-only wallet. Connect a seed or keypair to run cleanup.'
          ) : plan ? (
            `Swap: ${plan.swapCount} · Burn: ${plan.burnCount} · Close: ${plan.closeCount}. Estimate before fees.`
          ) : (
            'A fresh plan is required before cleanup.'
          )}
        </div>
      )}
      <RecoverButton
        disabled={selectedCount === 0 || (!preview && (preparing || !plan?.canExecute || !canSign))}
        onClick={preview ? onPreviewComplete : onExecute}
        aria-label="RECOVER SOL"
      />
    </div>
  );
}
