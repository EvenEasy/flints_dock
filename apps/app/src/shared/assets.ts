// The design catalogs are bundled locally; wallet metadata cannot select arbitrary image URLs.
const files = import.meta.glob<string>('../../assets/design/*.svg', {
  eager: true,
  query: '?url',
  import: 'default',
});

export type DesignPage = 'welcome' | 'scan' | 'main' | 'cleanup' | 'success';
export type Tone = 'cyan' | 'purple' | 'green' | 'red' | 'amber' | 'muted';
export type IconName =
  'wallet' | 'warning' | 'info' | 'rocket' | 'home' | 'cargo' | 'target' | 'skull';

/** Resolve an application-owned XML catalog key to an inert, bundled SVG image. */
export function designAsset(page: DesignPage, key: string): string {
  const asset = files[`../../assets/design/${page}-${key}.svg`];
  if (!asset) throw new Error(`Missing local design asset: ${page}/${key}`);
  return asset;
}
