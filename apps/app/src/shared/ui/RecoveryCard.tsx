import { Icon } from './Icon';

/** Values are backend-provided estimates or explicitly labelled reference values, never quotes calculated in JS. */
export function RecoveryCard({
  label,
  value,
  dollars,
  unavailable = false,
}: {
  label: string;
  value: string;
  dollars?: string;
  unavailable?: boolean;
}) {
  return (
    <section
      className={`recovery-card ${unavailable ? 'recovery-card--unavailable' : ''}`}
      aria-label={label}
    >
      <Icon name="salvage-coins" />
      <div>
        <p className="eyebrow">{label}</p>
        <strong>{value}</strong>
        {dollars && <p className="recovery-card__usd">{dollars}</p>}
      </div>
    </section>
  );
}
