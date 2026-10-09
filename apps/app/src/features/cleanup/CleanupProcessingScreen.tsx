import type { CleanupProgress } from '../../../frontend-contract/cleanup';
import { Brand, MascotHero, MechanicalPanel } from '../../shared/ui/Design';
import { Notice } from '../../shared/ui/Notice';
const stages = [
  ['swap', 'Swap'],
  ['burn', 'Burning'],
  ['close', 'Closing'],
  ['confirmation', 'Confirmation'],
  ['accounting', 'Accounting'],
] as const;
/** Every displayed count comes from Rust; leaving this screen does not cancel a transaction. */
export function CleanupProcessingScreen({
  progress,
  skippedStages = [],
  error,
  onRecover,
}: {
  progress: CleanupProgress | null;
  skippedStages?: string[];
  error: string | null;
  onRecover?: () => void;
}) {
  return (
    <div className="page page--scan page--processing" aria-busy={!error}>
      <Brand />
      <MascotHero scanning />
      <MechanicalPanel className="scan-title">
        <h1 className="type-screen" id="screen-heading" tabIndex={-1}>
          WALLET CLEANUP
        </h1>
      </MechanicalPanel>
      <MechanicalPanel tone="cyan" className="scan-process">
        <p role="status">
          {progress
            ? `${progress.completed} / ${progress.total} accounts · ${stages.find(([key]) => key === progress.stage)?.[1] ?? progress.stage}`
            : 'Checking the approved plan'}
        </p>
        {progress?.account && (
          <p className="type-caption progress-account" title={progress.account}>
            {progress.account}
          </p>
        )}
        {skippedStages.length > 0 && (
          <p className="type-caption">
            Skipped:{' '}
            {skippedStages
              .map((stage) => stages.find(([key]) => key === stage)?.[1] ?? stage)
              .join(', ')}
          </p>
        )}
        <p className="type-caption">Submitted transactions await confirmation.</p>
        {error && (
          <Notice tone="red" alert>
            {error}
            <button className="text-button" type="button" onClick={onRecover}>
              REFRESH STATUS
            </button>
          </Notice>
        )}
      </MechanicalPanel>
    </div>
  );
}
