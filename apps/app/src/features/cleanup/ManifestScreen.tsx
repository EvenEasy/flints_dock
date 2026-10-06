import { InventoryDetails } from '../wallet/InventoryDetails';
import { ScreenTitle } from '../../shared/ui/ScreenTitle';
import { Icon } from '../../shared/ui/Icon';
import { ActionButton } from '../../shared/ui/ActionButton';
import { RecoveryCard } from '../../shared/ui/RecoveryCard';
import { Notice } from '../../shared/ui/Notice';
import type { ScreenProps } from '../../app/ScreenProps';
import type { IconName, Tone } from '../../shared/assets';

/** The manifest exposes backend capability gaps instead of classifying assets in JavaScript. */
export function ManifestScreen({ preview, analysis, onNavigate }: ScreenProps) {
  const rows: {
    label: string;
    detail: string;
    value: string;
    icon: IconName;
    tone: Tone;
    target?: 'tokens' | 'dead' | 'nfts';
  }[] = [
    {
      label: 'SWAPPABLE TOKENS',
      detail: preview ? '18 SELECTED' : 'ROUTE PLAN UNAVAILABLE',
      value: preview ? '≈ 0.028 SOL' : 'Inspect inventory',
      icon: 'swap',
      tone: 'purple',
      target: 'tokens',
    },
    {
      label: 'DEAD TOKENS',
      detail: preview ? '23 TO BURN' : 'BURN ASSESSMENT UNAVAILABLE',
      value: preview ? 'REMOVE JUNK' : 'No burn requested',
      icon: 'skull',
      tone: 'red',
      target: 'dead',
    },
    {
      label: 'EMPTY ACCOUNTS',
      detail: preview
        ? '12 TO CLOSE'
        : `${analysis?.accountSummary?.emptyTokenAccounts ?? '—'} DISCOVERED`,
      value: preview ? '≈ 0.024 SOL' : 'Close API unavailable',
      icon: 'accounts',
      tone: 'cyan',
    },
    {
      label: 'NFTS',
      detail: preview ? '7 KEPT UNTOUCHED' : 'VIEW ONLY',
      value: 'LEAVE UNTOUCHED',
      icon: 'nft',
      tone: 'purple',
      target: 'nfts',
    },
  ];
  return (
    <>
      <ScreenTitle subtitle="REVIEW YOUR SALVAGE MANIFEST">CLEANUP MANIFEST</ScreenTitle>
      {!preview && (
        <Notice tone="amber" title="PLANNING API REQUIRED">
          Inventory is available. A safe cleanup plan must come from the backend before any action
          can be confirmed.
        </Notice>
      )}
      <div className="manifest-list">
        {rows.map((row) => (
          <button
            type="button"
            key={row.label}
            className={`manifest-row manifest-row--${row.tone}`}
            disabled={!row.target}
            onClick={() => row.target && onNavigate(row.target)}
          >
            <Icon name={row.icon} tone={row.tone} />
            <div>
              <strong>{row.label}</strong>
              <small>{row.detail}</small>
            </div>
            <span>{row.value}</span>
            {row.target && <Icon name="chevron" />}
          </button>
        ))}
      </div>
      {!preview && analysis && <InventoryDetails analysis={analysis} />}
      <RecoveryCard
        label="ESTIMATED RECOVERY"
        value={preview ? '≈ 0.052 SOL' : 'UNAVAILABLE'}
        dollars={preview ? '≈ $8.64 USD' : 'No execution or quotes requested'}
        unavailable={!preview}
      />
      <ActionButton icon="raptor-action" onClick={() => onNavigate('confirm')}>
        {preview ? 'CLEAN WALLET' : 'REVIEW REQUIRED CAPABILITIES'}
      </ActionButton>
      <p className="micro-note">
        {preview
          ? 'Reference sequence only. No wallet is connected.'
          : 'Your marked mints are local display preferences until the planning contract supports them.'}
      </p>
    </>
  );
}
