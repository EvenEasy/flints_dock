import { ScreenTitle } from '../../shared/ui/ScreenTitle';
import { Icon } from '../../shared/ui/Icon';
import { ActionButton } from '../../shared/ui/ActionButton';
import { Notice } from '../../shared/ui/Notice';
import { salvageSteps } from '../preview/designData';
import type { ScreenProps } from '../../app/ScreenProps';

/** Static operation stages are restricted to reference preview; live progress needs backend events. */
export function SalvageScreen({ preview, onNavigate }: ScreenProps) {
  return (
    <>
      <ScreenTitle
        subtitle={preview ? 'PROCESSING YOUR SALVAGE MANIFEST' : 'NO OPERATION IS RUNNING'}
      >
        SALVAGE OPERATION
      </ScreenTitle>
      {preview ? (
        <>
          <div className="operation-heading">
            <span>IN PROGRESS</span>
            <strong>7 / 18 ASSETS</strong>
          </div>
          <div
            className="progress-track progress-track--salvage"
            role="progressbar"
            aria-label="Reference cleanup progress"
            aria-valuemin={0}
            aria-valuemax={18}
            aria-valuenow={7}
          >
            <div className="progress-fill" />
          </div>
          <ol className="operation-steps salvage-steps">
            {salvageSteps.map((step, index) => (
              <li
                key={step}
                className={
                  index < 4 ? 'step--done' : index === 4 ? 'step--active' : 'step--pending'
                }
              >
                <Icon
                  name={index < 4 ? 'check-circle' : index === 4 ? 'spinner' : 'pending'}
                  tone={index < 4 ? 'green' : index === 4 ? 'purple' : 'muted'}
                />
                <span>{step}</span>
                <span className="sr-only">
                  {index < 4
                    ? 'Complete in reference'
                    : index === 4
                      ? 'In progress in reference'
                      : 'Pending in reference'}
                </span>
              </li>
            ))}
          </ol>
          <Notice>
            Reference: a cleanup may require multiple transactions. Real progress must follow
            confirmed backend events.
          </Notice>
          <ActionButton variant="secondary" onClick={() => onNavigate('success')}>
            PREVIEW COMPLETION
          </ActionButton>
        </>
      ) : (
        <Notice tone="amber">
          Execution and transaction progress are not exposed by the backend. No transactions have
          been submitted.
        </Notice>
      )}
    </>
  );
}
