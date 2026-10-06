import { describe, expect, it } from 'vitest';
import { decimal, lamportsToSol, looksLikeAddress, usd } from './format';

describe('Exact presentation formatting', () => {
  it('Decimal_LargeExactString_PreservesAllDigits', () => {
    expect(decimal('900719925474099312345.000000001')).toBe(
      '900,719,925,474,099,312,345.000000001',
    );
  });
  it('Lamports_OneLamport_FormatsWithoutFloatingPoint', () => {
    expect(lamportsToSol('1')).toBe('0.000000001');
    expect(lamportsToSol('2039280')).toBe('0.00203928');
  });
  it('Formatting_UnavailableValues_UsesUnavailableMarker', () => {
    expect(decimal(null)).toBe('—');
    expect(usd(undefined)).toBe('—');
    expect(looksLikeAddress('private-key-with-spaces')).toBe(false);
  });
});
