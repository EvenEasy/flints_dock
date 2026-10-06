import { ScreenTitle } from '../../shared/ui/ScreenTitle';
import { media } from '../../shared/assets';
import { Icon } from '../../shared/ui/Icon';
import { ActionButton } from '../../shared/ui/ActionButton';
import { RecoveryCard } from '../../shared/ui/RecoveryCard';
import { Notice } from '../../shared/ui/Notice';
import type { ScreenProps } from '../../app/ScreenProps';

/** Completion figures require a confirmed backend report; only preview displays reference totals. */
export function SuccessScreen({ preview, onNavigate }: ScreenProps) {
  return (
    <>
      <ScreenTitle
        subtitle={
          preview ? 'YOUR WALLET IS READY FOR THE NEXT VOYAGE' : 'NO CLEANUP HAS BEEN EXECUTED'
        }
        purple
      >
        CARGO HOLD CLEAN
      </ScreenTitle>
      {preview ? (
        <>
          <img
            className="success-art"
            src={media('illustrations/success-raptor.png')}
            alt="Raptor captain celebrating a cleared cargo hold"
            width="268"
            height="136"
          />
          <RecoveryCard label="RECOVERED" value="0.0498 SOL" dollars="≈ $8.27 USD" />
          <div className="success-list">
            <div>
              <Icon name="swap" tone="purple" />
              <span>18 TOKENS SWAPPED</span>
              <strong>≈ 0.028 SOL</strong>
            </div>
            <div>
              <Icon name="skull" tone="red" />
              <span>23 DEAD TOKENS BURNED</span>
              <Icon name="check" tone="green" />
            </div>
            <div>
              <Icon name="accounts" />
              <span>12 EMPTY ACCOUNTS CLOSED</span>
              <strong>≈ 0.024 SOL</strong>
            </div>
            <div>
              <Icon name="nft" tone="purple" />
              <span>7 NFTS LEFT UNTOUCHED</span>
              <Icon name="check" tone="green" />
            </div>
            <div>
              <Icon name="shield" tone="green" />
              <span>IGNORED ASSETS UNTOUCHED</span>
              <Icon name="check" tone="green" />
            </div>
          </div>
        </>
      ) : (
        <Notice tone="amber">
          A confirmed cleanup report is unavailable. This view does not imply that assets were
          swapped, burned, or closed.
        </Notice>
      )}
      <ActionButton onClick={() => onNavigate('summary')}>DONE</ActionButton>
    </>
  );
}
