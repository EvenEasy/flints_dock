import type {
  CleanupPlan,
  CleanupProgress,
  CleanupPolicy,
} from '../../../frontend-contract/cleanup';
import { lamportsToSol } from '../../shared/format';
import type { WalletAnalysis } from '../../../frontend-contract/types';
import { Brand, MascotHero, MechanicalPanel } from '../../shared/ui/Design';
import { RecoverButton } from '../../shared/ui/ActionButton';
import { AssetRow } from '../assets/AssetRow';
import { selectableAssets } from '../assets/presentation';
import { previewAssets } from '../preview/designData';
import { CategoryNotice, Notice } from '../../shared/ui/Notice';

const skipLabels: Record<string, string> = {
  ignored_mint: 'Protected mint',
  not_selected: 'Not selected',
  unknown_classification: 'Unknown asset kind',
  nft_unsupported: 'NFT cleanup unsupported',
  frozen: 'Frozen account',
  routing_unavailable: 'Routing unavailable on this network',
  provider_failure: 'Provider unavailable',
  unsupported_account_extension: 'Unsupported account extension',
  unsupported_mint_extension: 'Unsupported mint extension',
  close_authority_mismatch: 'Missing close authority',
  owner_mismatch: 'Owner changed',
  mint_unavailable: 'Mint evidence unavailable',
  invalid_state: 'Invalid account state',
  native_balance: 'WSOL requires unwrap',
  insufficient_liquidity: 'Insufficient liquidity',
  price_impact: 'Price impact exceeds limit',
  quote_expired: 'Quote expired',
};

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
  policy = 'auto',
  onPolicyChange,
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
  policy?: CleanupPolicy;
  onPolicyChange?: (policy: CleanupPolicy) => void;
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
        {!preview && (
          <label className="type-caption">
            Cleanup policy{' '}
            <select
              aria-label="Cleanup policy"
              value={policy}
              onChange={(event) => onPolicyChange?.(event.target.value as CleanupPolicy)}
              disabled={preparing}
            >
              <option value="auto">Auto: swap / no-route burn</option>
              <option
                value="explicitDiscard"
                disabled={
                  analysis?.categories?.network !==
                    'EtWTRABZaYq6iMfeYKouRu166VU2xqa1wcaWoxPkrZBG' &&
                  plan?.network !== 'EtWTRABZaYq6iMfeYKouRu166VU2xqa1wcaWoxPkrZBG'
                }
              >
                Devnet: discard selected
              </option>
            </select>
          </label>
        )}
      </header>
      <MechanicalPanel className="asset-manifest">
        <h2 className="manifest-columns">
          <span>ASSET</span>
          <span>{preview ? 'VALUE IN SOL' : 'SOL ESTIMATE / ACTION'}</span>
        </h2>
        {!preview && (
          <CategoryNotice status={analysis?.tokens?.status ?? analysis?.allTokens?.status} />
        )}
        {plan && plan.undecodableAccounts.length > 0 && (
          <details className="type-caption">
            <summary>Unreadable accounts: {plan.undecodableAccounts.length}</summary>
            {plan.undecodableAccounts.map((account) => (
              <p key={account.address}>
                {account.address} · {account.reason}
              </p>
            ))}
          </details>
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
                  value={(() => {
                    const total = (plan?.entries ?? [])
                      .filter((entry) => entry.mint === asset.mint)
                      .reduce((sum, entry) => sum + BigInt(entry.expectedOutLamports ?? '0'), 0n);
                    return total > 0n ? `${lamportsToSol(total.toString())} SOL` : 'NOT ESTIMATED';
                  })()}
                  action={
                    plan
                      ? [
                          ...new Set(
                            plan.entries
                              .filter((entry) => entry.mint === asset.mint)
                              .map((entry) => entry.action.toUpperCase()),
                          ),
                        ].join(' / ')
                      : preparing
                        ? 'PLANNING'
                        : undefined
                  }
                  reason={plan?.entries
                    .filter((entry) => entry.mint === asset.mint && entry.action === 'skip')
                    .map((entry) => skipLabels[entry.reasonCode] ?? 'Review required')
                    .filter((reason, index, all) => all.indexOf(reason) === index)
                    .join(' · ')}
                  details={plan?.entries
                    .filter((entry) => entry.mint === asset.mint)
                    .map(
                      (entry) =>
                        `${entry.account} · ${entry.program} · raw ${entry.rawAmount} · ${entry.reasonCode}: ${entry.reason} · close authority ${entry.closeAuthority ?? 'owner'} · extensions ${[...entry.accountExtensions, ...entry.mintExtensions].join(', ') || 'none'}`,
                    )
                    .join(' | ')}
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
          {preview ? 'SELECTED:' : 'Selected assets:'}{' '}
          <strong>
            {selectedCount} {selectedCount === 1 ? 'ASSET' : 'ASSETS'}
          </strong>
          {!preview && (
            <span className="account-counts type-caption">
              Executable accounts: {plan?.executableAccounts ?? '—'}
              <br />
              Skipped accounts: {plan?.skippedAccounts ?? '—'}
            </span>
          )}
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
