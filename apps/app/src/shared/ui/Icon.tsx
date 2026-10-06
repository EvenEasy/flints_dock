import { media } from '../assets';
import type { IconName, Tone } from '../assets';

/** Supplied SVG contours are loaded as inert images, never inserted as raw HTML. */
export function Icon({
  name,
  tone = 'cyan',
  className = '',
}: {
  name: IconName;
  tone?: Tone;
  className?: string;
}) {
  return (
    <img
      src={media(`icons/base/${name}.png`)}
      alt=""
      aria-hidden="true"
      className={`icon icon--${tone} ${className}`}
      width="24"
      height="24"
    />
  );
}
