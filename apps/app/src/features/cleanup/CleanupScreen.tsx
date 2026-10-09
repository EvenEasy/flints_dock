import type { WalletAnalysis } from '../../../frontend-contract/types';
import { Brand, MascotHero, MechanicalPanel } from '../../shared/ui/Design';
import { RecoverButton } from '../../shared/ui/ActionButton';
import { AssetRow } from '../assets/AssetRow';
import { selectableAssets } from '../assets/presentation';
import { previewAssets } from '../preview/designData';
import { CategoryNotice, Notice } from '../../shared/ui/Notice';

/** Selection is frontend intent only until the desktop API supplies a quote-backed cleanup plan. */
export function CleanupScreen({
  preview,
  analysis,
  ignoredMints,
  onToggleMint,
  onPreviewComplete,
}: {
  preview: boolean;
  analysis: WalletAnalysis | null;
  ignoredMints: ReadonlySet<string>;
  onToggleMint: (mint: string) => void;
  onPreviewComplete: () => void;
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
          ОЧИЩЕННЯ
        </h1>
        <p>Зніми галочку з активів, які хочеш залишити.</p>
      </header>
      <MechanicalPanel className="asset-manifest">
        <h2 className="manifest-columns">
          <span>АКТИВ</span>
          <span>ВАРТІСТЬ В SOL</span>
        </h2>
        {!preview && (
          <CategoryNotice status={analysis?.tokens?.status ?? analysis?.allTokens?.status} />
        )}
        <ul className="asset-list" aria-label="Активи для очищення">
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
                  value="НЕ ОЦІНЕНО"
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
          ВИБРАНО:{' '}
          <strong>
            {selectedCount} {selectedCount === 4 ? 'АКТИВИ' : 'АКТИВІВ'}
          </strong>
        </p>
        <div className="selection-estimate">
          <p>ОРІЄНТОВНЕ ПОВЕРНЕННЯ</p>
          <strong>{preview ? (selectedCount === 0 ? '—' : '≈ 0.428 SOL') : 'НЕ ОЦІНЕНО'}</strong>
        </div>
      </MechanicalPanel>
      {!preview && (
        <Notice tone="amber" title="ОЧИЩЕННЯ ПОКИ НЕДОСТУПНЕ">
          API очищення ще недоступне. Вибір збережено; активи не змінюються.
        </Notice>
      )}
      <RecoverButton
        disabled={!preview || selectedCount === 0}
        onClick={onPreviewComplete}
        aria-label={preview ? 'ПОВЕРНУТИ SOL' : 'ПОВЕРНУТИ SOL — очищення недоступне'}
      />
    </div>
  );
}
