import { Brand, MascotHero, MechanicalPanel } from '../../shared/ui/Design';
import { designAsset } from '../../shared/assets';
import { Notice } from '../../shared/ui/Notice';
import { ActionButton } from '../../shared/ui/ActionButton';
import type { ReadError } from '../../shared/api/wallet';

const steps = [
  'Connecting wallet',
  'Loading assets',
  'Checking tokens and NFTs',
  'Estimating reclaimable SOL',
  'Preparing results',
];

/** Preview reproduces stage three; live IPC is one pending snapshot, not five invented progress events. */
export function ScanScreen({
  preview,
  progress,
  error,
  onRetry,
  onReconnect,
  onDismiss,
}: {
  preview: boolean;
  progress?: import('../../../frontend-contract/cleanup').CleanupProgress | null;
  error: ReadError | null;
  onRetry: () => void;
  onReconnect: () => void;
  onDismiss: () => void;
}) {
  return (
    <div className="page page--scan" aria-busy={!preview && !error}>
      <Brand />
      <MascotHero scanning />
      <MechanicalPanel className="connected-status">
        <img src={designAsset('scan', 'wallet_outline_icon')} alt="" aria-hidden="true" />
        <span className="status-led status-led--connected" aria-hidden="true" />
        <p>WALLET CONNECTED</p>
      </MechanicalPanel>
      <MechanicalPanel className="scan-title">
        <h1 className="type-screen" id="screen-heading" tabIndex={-1}>
          {error ? 'ANALYSIS INTERRUPTED' : 'WALLET ANALYSIS'}
        </h1>
      </MechanicalPanel>
      <MechanicalPanel tone="cyan" className="scan-process">
        {error ? (
          <div className="scan-error">
            <Notice tone="red" alert title="ANALYSIS FAILED">
              {error.message}
            </Notice>
            <ActionButton onClick={onRetry}>RETRY SCAN</ActionButton>
            <button type="button" className="text-button" onClick={onReconnect}>
              CHANGE WALLET
            </button>
            <button type="button" className="text-button" onClick={onDismiss}>
              DISMISS SCAN
            </button>
          </div>
        ) : (
          <>
            <h2 className="type-section" role="status">
              {preview
                ? 'STEP 3 OF 5'
                : progress
                  ? ({
                      discovery: 'LOADING ASSETS',
                      classification: 'CHECKING ASSETS',
                      pricing: 'ESTIMATING VALUE',
                      'risk/routing/DAS': 'ROUTES AND RISK',
                      completed: 'DONE',
                    }[progress.stage] ?? 'FETCHING DATA')
                  : 'FETCHING DATA'}
            </h2>
            {preview && (
              <div
                className="stage-progress"
                role="progressbar"
                aria-label="Completed analysis stages"
                aria-valuemin={0}
                aria-valuemax={5}
                aria-valuenow={preview ? 2 : undefined}
                aria-valuetext={
                  preview
                    ? '2 of 5 stages complete; stage 3 in progress'
                    : 'Waiting for analysis results'
                }
              >
                {steps.map((step, index) => (
                  <img
                    key={step}
                    src={designAsset(
                      'scan',
                      `progress_capsule_${preview ? (index < 2 ? 'complete' : index === 2 ? 'active' : 'pending') : index === 0 ? 'complete' : 'pending'}`,
                    )}
                    alt=""
                    aria-hidden="true"
                  />
                ))}
              </div>
            )}
            {preview && (
              <ol className="analysis-steps">
                {steps.map((step, index) => {
                  const state = preview
                    ? index < 2
                      ? 'complete'
                      : index === 2
                        ? 'active'
                        : 'pending'
                    : index === 0
                      ? 'complete'
                      : index === 1
                        ? 'active'
                        : 'pending';
                  return (
                    <li key={step} data-state={state}>
                      <img
                        src={designAsset(
                          'scan',
                          state === 'complete'
                            ? 'step_check_circle'
                            : state === 'active'
                              ? 'step_active_spinner'
                              : 'step_pending_circle',
                        )}
                        alt=""
                        aria-hidden="true"
                      />
                      <span>{step}</span>
                      <strong>
                        {state === 'complete'
                          ? 'DONE'
                          : state === 'active'
                            ? 'IN PROGRESS'
                            : preview
                              ? 'PENDING'
                              : 'NO STAGE DATA'}
                      </strong>
                    </li>
                  );
                })}
              </ol>
            )}
            {!preview && progress && (
              <p role="status" className="type-caption">
                {progress.status} · {progress.completed} / {progress.total} stages
              </p>
            )}
            <p className="automatic-note">Results will open automatically</p>
            {!preview && (
              <button className="text-button scan-dismiss" type="button" onClick={onDismiss}>
                DISMISS SCAN
              </button>
            )}
          </>
        )}
      </MechanicalPanel>
    </div>
  );
}
