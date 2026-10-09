import { useId } from 'react';
import { designAsset } from '../../shared/assets';
import { Icon } from '../../shared/ui/Icon';
import { SurfaceFrame } from '../../shared/ui/Design';
import { ExactAmount } from '../../shared/ui/ExactAmount';

/** Checked means include; unchecked means keep. The row identity is always a mint or preview key. */
export function AssetRow({
  name,
  quantity,
  identity,
  value,
  selected,
  onToggle,
  art,
  dead = false,
}: {
  name: string;
  quantity: string;
  identity: string;
  value: string;
  selected: boolean;
  onToggle: () => void;
  art?: string;
  dead?: boolean;
}) {
  const quantityId = useId();
  return (
    <li className="asset-row" data-asset={identity}>
      <label className="selection-control">
        <input
          type="checkbox"
          checked={selected}
          onChange={onToggle}
          aria-label={`Include ${name} mint ${identity}`}
          aria-describedby={quantityId}
        />
        <img
          src={designAsset('cleanup', selected ? 'checkbox_checked' : 'checkbox_unchecked')}
          alt=""
          aria-hidden="true"
        />
      </label>
      <div className="asset-thumbnail">
        {art ? (
          <img src={designAsset('cleanup', art)} alt="" aria-hidden="true" />
        ) : (
          <Icon name="wallet" />
        )}
      </div>
      <div className="asset-identity" title={identity}>
        <strong>{name}</strong>
        <p id={quantityId}>{quantity}</p>
      </div>
      <div className="asset-valuation">
        <strong>
          {value.endsWith(' SOL') ? <ExactAmount amount={value.slice(0, -4)} size="row" /> : value}
        </strong>
        {!selected ? (
          <span className="keep-label">ЗАЛИШИТИ</span>
        ) : dead ? (
          <span className="dead-badge">
            <SurfaceFrame />
            <span>МЕРТВИЙ</span>
          </span>
        ) : null}
      </div>
    </li>
  );
}
