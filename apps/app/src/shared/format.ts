/** Format a decimal balance without ever coercing its exact digits to a JS number. */
export function decimal(value: string | null | undefined): string {
  if (value == null || !/^\d+(\.\d+)?$/.test(value)) return '—';
  const [whole = '0', fraction] = value.split('.');
  const grouped = whole.replace(/\B(?=(\d{3})+(?!\d))/g, ',');
  return fraction === undefined ? grouped : `${grouped}.${fraction}`;
}

/** Insert a decimal separator for presentation; no valuation or cleanup arithmetic occurs. */
export function lamportsToSol(value: string | null | undefined): string {
  if (value == null || !/^\d+$/.test(value)) return '—';
  const padded = value.padStart(10, '0');
  const fraction = padded.slice(-9).replace(/0+$/, '');
  return decimal(`${padded.slice(0, -9)}${fraction ? `.${fraction}` : ''}`);
}

export function usd(value: number | null | undefined): string {
  return value != null && Number.isFinite(value)
    ? new Intl.NumberFormat('en-US', { style: 'currency', currency: 'USD' }).format(value)
    : '—';
}

/** Preserve small unit prices instead of displaying a positive price as $0.00. */
export function usdUnitPrice(value: number | null | undefined): string {
  if (value == null || !Number.isFinite(value) || value < 0) return '—';

  // Scientific notation keeps tiny quotes readable in a narrow phone table.
  if (value > 0 && value < 1e-6)
    return new Intl.NumberFormat('en-US', {
      style: 'currency',
      currency: 'USD',
      notation: 'scientific',
      maximumSignificantDigits: 6,
    }).format(value);

  return new Intl.NumberFormat('en-US', {
    style: 'currency',
    currency: 'USD',
    minimumFractionDigits: 2,
    maximumFractionDigits: value < 1 ? 8 : 4,
  }).format(value);
}

export function shortAddress(value: string): string {
  return value.length > 16 ? `${value.slice(0, 6)}…${value.slice(-5)}` : value;
}

/** This is a UX-only shape check; Rust remains responsible for authoritative Pubkey validation. */
export function looksLikeAddress(value: string): boolean {
  return /^[1-9A-HJ-NP-Za-km-z]{32,44}$/.test(value);
}
