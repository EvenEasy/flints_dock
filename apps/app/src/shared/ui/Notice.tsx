import type { ReactNode } from 'react';
import { Icon } from './Icon';
import type { Tone } from '../assets';
import type { ScanStatus } from '../../../frontend-contract/types';

export function Notice({
  children,
  tone = 'cyan',
  title,
  alert = false,
  id,
}: {
  children: ReactNode;
  tone?: Tone;
  title?: string;
  alert?: boolean;
  id?: string;
}) {
  return (
    <div id={id} className={`notice notice--${tone}`} role={alert ? 'alert' : undefined}>
      <Icon name={tone === 'red' || tone === 'amber' ? 'warning' : 'info'} tone={tone} />
      <div>
        {title && <strong>{title}</strong>}
        <div>{children}</div>
      </div>
    </div>
  );
}

/** Missing/failed data is distinct from a successful empty category. */
export function CategoryNotice({ status }: { status: ScanStatus | undefined }) {
  if (!status || status.status === 'complete') return null;
  return (
    <Notice
      tone={status.status === 'failed' ? 'red' : 'amber'}
      title={status.status === 'partial' ? 'PARTIAL RESULTS' : status.status.toUpperCase()}
    >
      {status.reason}
    </Notice>
  );
}
