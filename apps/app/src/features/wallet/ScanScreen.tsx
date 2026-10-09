import { Brand, MascotHero, MechanicalPanel } from '../../shared/ui/Design';
import { designAsset } from '../../shared/assets';
import { Notice } from '../../shared/ui/Notice';
import { ActionButton } from '../../shared/ui/ActionButton';
import type { ReadError } from '../../shared/api/wallet';

const steps = [
  'Підключення гаманця',
  'Завантаження активів',
  'Перевірка токенів і NFT',
  'Розрахунок доступного SOL',
  'Підготовка результатів',
];

/** Preview reproduces stage three; live IPC is one pending snapshot, not five invented progress events. */
export function ScanScreen({
  preview,
  error,
  onRetry,
  onReconnect,
  onDismiss,
}: {
  preview: boolean;
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
        <p>ГАМАНЕЦЬ ПІДКЛЮЧЕНО</p>
      </MechanicalPanel>
      <MechanicalPanel className="scan-title">
        <h1 className="type-screen" id="screen-heading" tabIndex={-1}>
          {error ? 'АНАЛІЗ ПЕРЕРВАНО' : 'АНАЛІЗ ГАМАНЦЯ'}
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
              {preview ? 'ЕТАП 3 ІЗ 5' : 'ОТРИМУЄМО ДАНІ'}
            </h2>
            <div
              className="stage-progress"
              role="progressbar"
              aria-label="Завершені етапи аналізу"
              aria-valuemin={0}
              aria-valuemax={5}
              aria-valuenow={preview ? 2 : undefined}
              aria-valuetext={
                preview
                  ? 'Завершено 2 етапи з 5; виконується третій'
                  : 'Очікуємо готовий результат аналізу'
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
                        ? 'ГОТОВО'
                        : state === 'active'
                          ? 'ВИКОНУЄТЬСЯ'
                          : preview
                            ? 'ОЧІКУЄ'
                            : 'БЕЗ ДАНИХ ЕТАПУ'}
                    </strong>
                  </li>
                );
              })}
            </ol>
            <p className="automatic-note">Результати відкриються автоматично</p>
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
