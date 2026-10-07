import type { WalletConnection } from '../../../frontend-contract/types';
import { ScreenTitle } from '../../shared/ui/ScreenTitle';
import { Icon } from '../../shared/ui/Icon';
import { RecoveryCard } from '../../shared/ui/RecoveryCard';
import { ActionButton } from '../../shared/ui/ActionButton';
import { CategoryNotice, Notice } from '../../shared/ui/Notice';
import { decimal, usd } from '../../shared/format';
import type { IconName, Tone } from '../../shared/assets';
import type { ScreenProps } from '../../app/ScreenProps';

/** Risk/dust/dead labels remain unavailable until the backend exposes a classification/route plan. */
export function SummaryScreen({
  preview,
  analysis,
  onNavigate,
  connection,
}: ScreenProps & { connection?: WalletConnection | null }) {
  const nftCount =
    analysis?.nfts && (analysis.nfts.classic.items !== null || analysis.nfts.core.items !== null)
      ? (analysis.nfts.classic.items?.length ?? 0) + (analysis.nfts.core.items?.length ?? 0)
      : '—';
  const cards: {
    label: string;
    count: number | string;
    caption: string;
    tone: Tone;
    icon: IconName;
    target: 'tokens' | 'nfts' | 'dead' | 'manifest';
  }[] = [
    {
      label: 'SCAM',
      count: preview ? 42 : '—',
      caption: preview ? 'SUSPICIOUS TOKENS' : 'NOT CLASSIFIED',
      tone: 'red',
      icon: 'warning',
      target: 'tokens',
    },
    {
      label: 'NFT',
      count: preview ? 24 : nftCount,
      caption: 'COLLECTIBLES',
      tone: 'purple',
      icon: 'nft',
      target: 'nfts',
    },
    {
      label: 'DUST',
      count: preview ? 53 : '—',
      caption: preview ? 'LOW VALUE TOKENS' : 'NOT CLASSIFIED',
      tone: 'amber',
      icon: 'coins',
      target: 'tokens',
    },
    {
      label: 'DEAD TOKENS',
      count: preview ? 23 : '—',
      caption: preview ? 'NO LIQUIDITY' : 'ROUTE CHECK REQUIRED',
      tone: 'muted',
      icon: 'skull',
      target: 'dead',
    },
    {
      label: 'EMPTY ACCOUNTS',
      count: preview ? 12 : (analysis?.accountSummary?.emptyTokenAccounts ?? '—'),
      caption: preview ? 'CLOSED FOR RENT' : 'DISCOVERED, NOT CLOSED',
      tone: 'cyan',
      icon: 'accounts',
      target: 'manifest',
    },
  ];
  return (
    <>
      <ScreenTitle
        subtitle={
          preview ? 'WE FOUND SOME JUNK IN YOUR CARGO HOLD' : 'YOUR READ-ONLY WALLET INVENTORY'
        }
      >
        SCAN COMPLETE
      </ScreenTitle>
      {!preview && analysis?.balance && (
        <div className="balance-strip">
          <span>NATIVE SOL</span>
          <strong>{decimal(analysis.balance.value?.sol)} SOL</strong>
          <span>{usd(analysis.balance.value?.valueUsd)}</span>
        </div>
      )}
      {!preview && !analysis?.hasUsableResults && (
        <Notice tone="red" alert>
          No usable asset results. Inspect the category diagnostics below or rescan.
        </Notice>
      )}
      {!preview && connection && (
        <p className="wallet-capability">
          {connection.canSign ? 'LOCAL SIGNER CONNECTED' : 'PUBLIC ADDRESS · READ ONLY'}
          <small>{connection.walletAddress}</small>
        </p>
      )}
      <div className="summary-grid">
        {cards.map((card) => (
          <button
            type="button"
            key={card.label}
            className={`metric-card metric-card--${card.tone}`}
            onClick={() => onNavigate(card.target)}
          >
            <span className="metric-card__label">{card.label}</span>
            <Icon name={card.icon} tone={card.tone} />
            <strong>{card.count}</strong>
            <span className="metric-card__caption">{card.caption}</span>
          </button>
        ))}
      </div>
      {!preview && (
        <>
          <CategoryNotice status={analysis?.balance?.status} />
          <CategoryNotice status={analysis?.tokens?.status} />
          <CategoryNotice status={analysis?.scanners.prices} />
        </>
      )}
      <RecoveryCard
        label="ESTIMATED RECOVERABLE"
        value={preview ? '0.052 SOL' : 'PLAN UNAVAILABLE'}
        dollars={preview ? '≈ $8.64 USD' : 'Requires a backend cleanup plan'}
        unavailable={!preview}
      />
      <ActionButton onClick={() => onNavigate('manifest')}>
        REVIEW CLEANUP <Icon name="chevrons" />
      </ActionButton>
    </>
  );
}
