import { media } from '../../shared/assets';
import { Icon } from '../../shared/ui/Icon';
import { ActionButton } from '../../shared/ui/ActionButton';
import { Notice } from '../../shared/ui/Notice';
import type { ReadError } from '../../shared/api/wallet';

/** Entry artwork and benefits follow screen s01; connection accepts a public address only. */
export function WelcomeScreen({
  onConnect,
  onPreview,
  preview,
  error,
}: {
  onConnect: () => void;
  onPreview: () => void;
  preview: boolean;
  error: ReadError | null;
}) {
  return (
    <div className="welcome-screen">
      <header className="welcome-brand">
        <img src={media('brand/pirate-emblem.png')} alt="" width="62" height="51" />
        <h1 tabIndex={-1}>FLINT’S DOCK</h1>
        <p>ORBITAL SOLANA CLEANUP STATION</p>
      </header>
      <div className="welcome-art">
        <img
          src={media('illustrations/welcome-raptor.png')}
          alt="Raptor pirate at an orbital Solana cleanup station"
          width="281"
          height="237"
          fetchPriority="high"
        />
        <p>
          Scan your wallet.
          <br />
          Clean junk assets.
          <br />
          Recover useful SOL.
        </p>
      </div>
      <div className="benefits">
        <div>
          <Icon name="coins" />
          <span>
            FIND
            <br />
            JUNK
          </span>
        </div>
        <div>
          <Icon name="trash" />
          <span>
            REMOVE
            <br />
            TRASH
          </span>
        </div>
        <div>
          <Icon name="salvage-coins" />
          <span>
            RECOVER
            <br />
            SOL
          </span>
        </div>
      </div>
      {error && (
        <Notice tone="red" alert title="ANALYSIS UNAVAILABLE">
          {error.message}
        </Notice>
      )}
      <ActionButton icon="wallet" onClick={onConnect}>
        CONNECT WALLET
      </ActionButton>
      {!preview && (
        <button type="button" className="text-button" onClick={onPreview}>
          EXPLORE DESIGN PREVIEW <span aria-hidden="true">↗</span>
        </button>
      )}
    </div>
  );
}
