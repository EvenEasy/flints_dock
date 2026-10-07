import { describe, expect, it } from 'vitest';
import { displayOptions, phoneWidth } from './display';

describe('Phone presentation options', () => {
  it('Defaults_NoOptions_Uses390pxWithoutBackdrop', () => {
    expect(displayOptions('')).toEqual({ width: 390, backdrop: false });
  });
  it.each([
    ['320', 360],
    ['360', 360],
    ['393', 393],
    ['430', 430],
    ['480', 480],
    ['700', 480],
    ['NaN', 390],
  ])('Width_%s_BoundsTo%s', (value, expected) => {
    expect(phoneWidth(value)).toBe(expected);
  });
  it('URL_ExplicitPreferences_OverridesLaunchDefaults', () => {
    expect(displayOptions('?width=430&backdrop=0', { width: '393', backdrop: '1' })).toEqual({
      width: 430,
      backdrop: false,
    });
  });
});
