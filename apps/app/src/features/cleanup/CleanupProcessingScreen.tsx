import type { CleanupProgress } from '../../../frontend-contract/cleanup';
import { Brand, MascotHero, MechanicalPanel } from '../../shared/ui/Design';
import { Notice } from '../../shared/ui/Notice';
const stages = [
  ['swap', 'Свап'],
  ['burn', 'Спалювання'],
  ['close', 'Закриття'],
  ['confirmation', 'Підтвердження'],
  ['accounting', 'Облік'],
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
          ОЧИЩЕННЯ ГАМАНЦЯ
        </h1>
      </MechanicalPanel>
      <MechanicalPanel tone="cyan" className="scan-process">
        <p role="status">
          {progress
            ? `${progress.completed} / ${progress.total} акаунтів · ${stages.find(([key]) => key === progress.stage)?.[1] ?? progress.stage}`
            : 'Перевіряємо погоджений план'}
        </p>
        {progress?.account && (
          <p className="type-caption progress-account" title={progress.account}>
            {progress.account}
          </p>
        )}
        {skippedStages.length > 0 && (
          <p className="type-caption">
            Пропущено:{' '}
            {skippedStages
              .map((stage) => stages.find(([key]) => key === stage)?.[1] ?? stage)
              .join(', ')}
          </p>
        )}
        <p className="type-caption">Після відправлення чекаємо підтвердження.</p>
        {error && (
          <Notice tone="red" alert>
            {error}
            <button className="text-button" type="button" onClick={onRecover}>
              ОНОВИТИ СТАТУС
            </button>
          </Notice>
        )}
      </MechanicalPanel>
    </div>
  );
}
