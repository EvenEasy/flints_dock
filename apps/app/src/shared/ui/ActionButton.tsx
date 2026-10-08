import type { ButtonHTMLAttributes, ReactNode } from 'react';
import { designAsset } from '../assets';
import type { IconName } from '../assets';
import { SurfaceFrame } from './Design';
import { Icon } from './Icon';

/** A single native target owns the mechanical frame, icon, and live action label. */
export function ActionButton({
  children,
  icon,
  className = '',
  variant = 'primary',
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
      <SurfaceFrame page="cleanup" asset="recover_action_shell" />
      <span className="action-content">
        {icon && <Icon name={icon} />}
        <span>{children}</span>
      </span>
    </button>
  );
}

export function RecoverButton({
  children = 'ПОВЕРНУТИ SOL',
  page = 'cleanup',
  ...props
}: ButtonHTMLAttributes<HTMLButtonElement> & { page?: 'main' | 'cleanup' }) {
  return (
    <button type="button" className="recover-button" {...props}>
      <SurfaceFrame
        page={page}
        asset={page === 'main' ? 'frame_recover_action' : 'recover_action_shell'}
      />
      <span>
        <img src={designAsset('cleanup', 'rocket_action_icon')} alt="" aria-hidden="true" />
        <span>{children}</span>
      </span>
    </button>
  );
}
