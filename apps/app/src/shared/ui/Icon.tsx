import { designAsset } from '../assets';
import type { IconName, Tone } from '../assets';

const symbols: Partial<Record<IconName, string>> = {
  wallet: 'wallet_outline_icon',
};
const navigation = {
  rocket: 'nav_rocket_icon',
  home: 'nav_home_icon',
  cargo: 'nav_cargo_icon',
  target: 'nav_target_icon',
  skull: 'nav_skull_icon',
} as const;

/** Local navigation artwork stays decorative; the adjacent text supplies its accessible name. */
export function Icon({
  name,
  tone = 'cyan',
  className = '',
}: {
  name: IconName;
  tone?: Tone;
  className?: string;
}) {
  if (name === 'info' || name === 'warning')
    return (
      <svg
        className={`icon icon--${tone} ${className}`}
        viewBox="0 0 24 24"
        aria-hidden="true"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.6"
      >
        {name === 'info' ? <circle cx="12" cy="12" r="9" /> : <path d="M12 3 22 21H2Z" />}
        <path d="M12 8v6m0 3v1" />
      </svg>
    );
  const src =
    name === 'wallet'
      ? designAsset('welcome', symbols.wallet!)
      : designAsset('cleanup', navigation[name]);
  return (
    <img
      src={src}
      alt=""
      aria-hidden="true"
      className={`icon icon--${tone} ${className}`}
      width="24"
      height="24"
    />
  );
}
