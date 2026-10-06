import type { ButtonHTMLAttributes, ReactNode } from 'react';
import { Icon } from './Icon';
import type { IconName } from '../assets';

/** One consistent chamfered action surface for primary, danger, and secondary controls. */
export function ActionButton({
  children,
  icon,
  variant = 'primary',
  className = '',
  ...props
}: ButtonHTMLAttributes<HTMLButtonElement> & {
  children: ReactNode;
  icon?: IconName;
  variant?: 'primary' | 'danger' | 'secondary';
}) {
  return (
    <button
      type="button"
      className={`action-button action-button--${variant} ${className}`}
      {...props}
    >
      {icon && <Icon name={icon} />}
      <span>{children}</span>
    </button>
  );
}
