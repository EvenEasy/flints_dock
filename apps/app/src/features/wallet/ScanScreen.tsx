import { media } from '../../shared/assets';
import { ScreenTitle } from '../../shared/ui/ScreenTitle';
import { Icon } from '../../shared/ui/Icon';
import { ActionButton } from '../../shared/ui/ActionButton';
import { scanSteps } from '../preview/designData';
import type { ScreenProps } from '../../app/ScreenProps';

/** Live scans are indeterminate: the current IPC returns a snapshot, not stage/progress events. */
export function ScanScreen({ preview, onNavigate }: ScreenProps) {
  return (
    <>
      <ScreenTitle subtitle="ANALYZING YOUR CARGO HOLD…">SCANNING WALLET</ScreenTitle>
      <div className="scan-art">
        <img
          src={media('illustrations/orbital-station.png')}
          alt="Orbital station above a blue planet"
          width="249"
          height="199"
        />
      </div>
      <div className="scan-progress">
        <div
          className={`progress-track ${!preview ? 'progress-track--indeterminate' : ''}`}
          role="progressbar"
          aria-label="Wallet scan progress"
          aria-valuemin={0}
          aria-valuemax={100}
          aria-valuenow={preview ? 67 : undefined}
        >
          <div className="progress-fill" />
        </div>
        <strong>{preview ? '67%' : '…'}</strong>
      </div>
      {preview ? (
        <ol className="operation-steps">
          {scanSteps.map((step, index) => (
            <li
              key={step}
              className={index < 2 ? 'step--done' : index === 2 ? 'step--active' : 'step--pending'}
            >
              <Icon
                name={index < 2 ? 'check-circle' : index === 2 ? 'spinner' : 'pending'}
                tone={index === 2 ? 'purple' : index > 2 ? 'muted' : 'cyan'}
              />
              {step}
              <span className="sr-only">
                {index < 2
                  ? 'Complete in reference'
                  : index === 2
                    ? 'In progress in reference'
                    : 'Pending in reference'}
              </span>
            </li>
          ))}
        </ol>
      ) : (
        <div className="live-scan-status" role="status">
          <Icon name="spinner" />
          <p>
            Reading selected wallet categories…
            <br />
            <span>
              RPC results and price availability will appear together. Route checking is not part of
              this API.
            </span>
          </p>
        </div>
      )}
      {preview && (
        <ActionButton variant="secondary" onClick={() => onNavigate('summary')}>
          PREVIEW SCAN RESULTS
        </ActionButton>
      )}
    </>
  );
}
