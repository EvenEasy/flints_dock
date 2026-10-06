import type { ReactNode } from 'react';

export function ScreenTitle({
  children,
  subtitle,
  purple = false,
}: {
  children: ReactNode;
  subtitle: string;
  purple?: boolean;
}) {
  return (
    <div className={`screen-title ${purple ? 'screen-title--purple' : ''}`}>
      <h1 tabIndex={-1}>{children}</h1>
      <p>{subtitle}</p>
    </div>
  );
}
