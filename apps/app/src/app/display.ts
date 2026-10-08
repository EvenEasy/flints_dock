export const PHONE_MIN_WIDTH = 360;
export const PHONE_MAX_WIDTH = 480;
export const PHONE_DEFAULT_WIDTH = 430;

export interface DisplayOptions {
  width: number;
  backdrop: boolean;
}

/** Bound presentation sizes; physical inches cannot be inferred reliably from CSS pixels. */
export function phoneWidth(value: string | undefined | null): number {
  if (!value || !/^\d+$/.test(value)) return PHONE_DEFAULT_WIDTH;
  return Math.max(PHONE_MIN_WIDTH, Math.min(PHONE_MAX_WIDTH, Number(value)));
}

/** URL overrides are useful for screenshots; launch defaults contain display settings only. */
export function displayOptions(
  search: string,
  defaults: { width?: string; backdrop?: string } = {},
): DisplayOptions {
  const query = new URLSearchParams(search);
  return {
    width: phoneWidth(query.get('width') ?? defaults.width),
    backdrop: (query.get('backdrop') ?? defaults.backdrop ?? '1') !== '0',
  };
}
