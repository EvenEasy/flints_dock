import { ScreenTitle } from '../../shared/ui/ScreenTitle';
import { ActionButton } from '../../shared/ui/ActionButton';
import { Icon } from '../../shared/ui/Icon';
import { Notice } from '../../shared/ui/Notice';
import type { ScreenProps } from '../../app/ScreenProps';

/** Confirmation cannot submit transactions: execution is absent from the immutable IPC contract. */
export function ConfirmScreen({ preview, onNavigate }: ScreenProps) {
  return (
    <>
      <ScreenTitle subtitle="FINAL SALVAGE CHECK BEFORE CLEANUP">CONFIRM CLEANUP</ScreenTitle>
      <div className="confirmation-list">
        <div>
          <Icon name="swap" tone="purple" />
          <section>
            <h2>SWAP TOKENS</h2>
            <p>
              {preview ? '18 tokens → SOL via Jupiter' : 'Fresh quotes and signing API required'}
            </p>
          </section>
          <strong>{preview ? '≈ 0.028 SOL' : '—'}</strong>
        </div>
        <div>
          <Icon name="fire" tone="red" />
          <section>
            <h2>BURN DEAD TOKENS</h2>
            <p>
              {preview
                ? '23 tokens · permanently removed'
                : 'Backend burnability assessment required'}
            </p>
          </section>
        </div>
        <div>
          <Icon name="accounts" />
          <section>
            <h2>CLOSE EMPTY ACCOUNTS</h2>
            <p>
              {preview ? '12 accounts · recover rent' : 'Backend closure and confirmation required'}
            </p>
          </section>
          <strong>{preview ? '≈ 0.024 SOL' : '—'}</strong>
        </div>
        <div>
          <Icon name="shield" tone="green" />
          <section>
            <h2>IGNORED ASSETS</h2>
            <p>
              {preview
                ? 'Protected tokens and NFTs stay untouched'
                : 'Mint-based exclusions must be enforced by the backend'}
            </p>
          </section>
        </div>
      </div>
      <Notice tone="amber" title="BURNING IS PERMANENT">
        Burned tokens cannot be recovered. Review every asset before authorizing a future cleanup.
      </Notice>
      {!preview && (
        <Notice title="EXECUTION NOT AVAILABLE">
          The current contract supports read-only analysis. No signing material or mutation command
          is requested by this interface.
        </Notice>
      )}
      <ActionButton
        variant="danger"
        icon="warning"
        disabled={!preview}
        onClick={() => onNavigate('salvage')}
      >
        {preview ? 'CONFIRM CLEANUP' : 'CLEANUP API REQUIRED'}
      </ActionButton>
      {preview && (
        <p className="micro-note">
          This button advances a reference screen. It does not authorize or execute a cleanup.
        </p>
      )}
    </>
  );
}
