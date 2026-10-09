import { Brand, MascotHero, MechanicalPanel } from '../../shared/ui/Design';
import { designAsset } from '../../shared/assets';
import { ExactAmount } from '../../shared/ui/ExactAmount';

/** A result payload is required; mounting this static view never starts an operation. */
export function SuccessScreen({ displayAmount }: { displayAmount: string }) {
  return (
    <div className="page page--success">
      <Brand />
      <MascotHero />
      <MechanicalPanel className="success-title">
        <h1 className="type-screen" id="screen-heading" tabIndex={-1}>
          ОЧИЩЕННЯ ЗАВЕРШЕНО
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
        <img
          className="medallion-check"
          src={designAsset('success', 'success_check_disk')}
          alt=""
        />
      </div>
      <MechanicalPanel tone="cyan" className="returned-sol">
        <p>ПОВЕРНУТО У ГАМАНЕЦЬ</p>
        <strong>
          <ExactAmount
            amount={displayAmount.endsWith(' SOL') ? displayAmount.slice(0, -4) : displayAmount}
            size="returned"
          />
        </strong>
      </MechanicalPanel>
      <div className="success-spacer" aria-hidden="true" />
    </div>
  );
}
