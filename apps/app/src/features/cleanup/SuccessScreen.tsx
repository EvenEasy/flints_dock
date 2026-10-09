import type { CleanupJob } from '../../../frontend-contract/cleanup';
import { Brand, MascotHero, MechanicalPanel } from '../../shared/ui/Design';
import { designAsset } from '../../shared/assets';
import { ExactAmount } from '../../shared/ui/ExactAmount';

/** A result payload is required; mounting this static view never starts an operation. */
export function SuccessScreen({
  displayAmount,
  job,
}: {
  displayAmount: string;
  job?: CleanupJob | null;
}) {
  return (
    <div className="page page--success">
      <Brand />
      <MascotHero />
      <MechanicalPanel className="success-title">
        <h1 className="type-screen" id="screen-heading" tabIndex={-1}>
          {job && job.status !== 'completed'
            ? job.status === 'partial'
              ? 'ОЧИЩЕННЯ ЧАСТКОВЕ'
              : 'ОЧИЩЕННЯ НЕ ЗАВЕРШЕНО'
            : 'ОЧИЩЕННЯ ЗАВЕРШЕНО'}
        </h1>
      </MechanicalPanel>
      <div className="success-medallion" aria-hidden="true">
        <img
          className="medallion-housing"
          src={designAsset('success', 'success_medallion_housing')}
          alt=""
        />
        <img
          className="medallion-pivot medallion-pivot--left"
          src={designAsset('success', 'medallion_side_pivot')}
          alt=""
        />
        <img
          className="medallion-pivot medallion-pivot--right"
          src={designAsset('success', 'medallion_side_pivot')}
          alt=""
        />
        {!job || job.status === 'completed' ? (
          <img
            className="medallion-check"
            src={designAsset('success', 'success_check_disk')}
            alt=""
          />
        ) : (
          <span className="medallion-status">!</span>
        )}
      </div>
      <MechanicalPanel tone="cyan" className="returned-sol">
        <p>
          {!job
            ? 'ПОВЕРНУТО У ГАМАНЕЦЬ'
            : job?.report?.accounting_complete === false
              ? 'ВІДОМА ЗМІНА SOL · ОБЛІК НЕПОВНИЙ'
              : 'ЧИСТА ЗМІНА SOL У ГАМАНЦІ'}
        </p>
        <strong>
          <ExactAmount
            amount={displayAmount.endsWith(' SOL') ? displayAmount.slice(0, -4) : displayAmount}
            size="returned"
          />
        </strong>
      </MechanicalPanel>
      {job && (
        <details className="cleanup-report type-caption">
          <summary>
            Звіт · закрито {job.report?.closed ?? 0} · помилок {job.report?.failed ?? 0}
          </summary>
          {job.error && <p>{job.error}</p>}
          {job.report?.results.map((result) => (
            <div key={result.token_account}>
              <p title={result.token_account}>
                {result.mint} · {result.status}
              </p>
              <p>{result.reason}</p>
              {result.operations.map((receipt) => (
                <p className="progress-account" key={receipt.signature} title={receipt.signature}>
                  {receipt.operation}: {receipt.signature}
                </p>
              ))}
              {result.uncertain_signature && (
                <p>Невизначена signature: {result.uncertain_signature}</p>
              )}
            </div>
          ))}
        </details>
      )}
      <div className="success-spacer" aria-hidden="true" />
    </div>
  );
}
