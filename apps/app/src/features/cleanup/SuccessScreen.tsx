import { Brand, MascotHero, MechanicalPanel } from '../../shared/ui/Design';
import { designAsset } from '../../shared/assets';

/** A result payload is required; mounting this static view never starts an operation. */
export function SuccessScreen({ displayAmount }: { displayAmount: string }) {
  return (
    <div className="page page--success">
      <Brand variant="success" />
      <MascotHero />
      <MechanicalPanel page="success" asset="success_title_shell" className="success-title">
        <h1 id="screen-heading" tabIndex={-1}>
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
      <MechanicalPanel page="success" asset="returned_sol_shell" className="returned-sol">
        <p>ПОВЕРНУТО У ГАМАНЕЦЬ</p>
        <strong>{displayAmount}</strong>
      </MechanicalPanel>
      <div className="success-spacer" aria-hidden="true" />
    </div>
  );
}
