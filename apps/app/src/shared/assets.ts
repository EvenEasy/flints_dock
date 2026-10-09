// Only application-owned resources can be resolved; wallet metadata never selects URLs.
const files = import.meta.glob<string>(
  [
    '../../assets/art/*.{png,webp}',
    '../../assets/design/*_icon.svg',
    '../../assets/design/main-flint_emblem.svg',
    '../../assets/design/main-pirate_skull_crossbones.svg',
    '../../assets/design/main-scan_radar.svg',
    '../../assets/design/main-cargo_cube.svg',
    '../../assets/design/scan-scan_beam_art.svg',
    '../../assets/design/scan-progress_capsule_*.svg',
    '../../assets/design/scan-step_*.svg',
    '../../assets/design/success-success_medallion_housing.svg',
    '../../assets/design/success-success_check_disk.svg',
    '../../assets/design/success-medallion_side_pivot.svg',
    '../../assets/design/cleanup-checkbox_*.svg',
    '../../assets/design/cleanup-*_coin.svg',
    '../../assets/design/cleanup-*_thumbnail.svg',
  ],
  { eager: true, query: '?url', import: 'default' },
);

export type DesignPage = 'welcome' | 'scan' | 'main' | 'cleanup' | 'success';
export type Tone = 'cyan' | 'purple' | 'green' | 'red' | 'amber' | 'muted';
export type IconName =
  'wallet' | 'warning' | 'info' | 'rocket' | 'home' | 'cargo' | 'target' | 'skull';

/** A semantic registry prevents page-specific copies and records composite artwork boundaries. */
export const mediaRegistry = {
  'hero.raptor': { file: 'art/hero-raptor-ring.png', includes: ['portrait', 'ring', 'badge'] },
  'scene.station': { file: 'art/station-scene.png', includes: ['planet', 'city', 'ships', 'dock'] },
  'decoration.sabers': { file: 'art/crossed-sabers.png', includes: ['sabers'] },
  'decoration.station': { file: 'art/orbital-station.png', includes: ['station'] },
  'category.scam': { file: 'art/category-scam.png', includes: ['skull'] },
  'category.nft': { file: 'art/category-nft.png', includes: ['coin'] },
  'category.dust': { file: 'art/category-dust.png', includes: ['crystals'] },
  'category.dead_token': { file: 'art/category-dead.png', includes: ['bone'] },
  'frame.violet': { file: 'art/frame-violet.png', includes: ['border'] },
  'frame.cyan': { file: 'art/frame-cyan.png', includes: ['border'] },
  'brand.emblem': { file: 'design/main-flint_emblem.svg', includes: ['emblem'] },
  'hero.scanBeam': { file: 'design/scan-scan_beam_art.svg', includes: ['beam'] },
  'navigation.station': { file: 'design/cleanup-nav_home_icon.svg', includes: ['icon'] },
  'navigation.hangar': { file: 'design/cleanup-nav_rocket_icon.svg', includes: ['icon'] },
  'navigation.holds': { file: 'design/cleanup-nav_cargo_icon.svg', includes: ['icon'] },
  'navigation.missions': { file: 'design/cleanup-nav_target_icon.svg', includes: ['icon'] },
  'navigation.profile': { file: 'design/cleanup-nav_skull_icon.svg', includes: ['icon'] },
} as const;

export type MediaKey = keyof typeof mediaRegistry;

/** Resolve a typed semantic key to a local PNG, WebP, or SVG without network requests. */
export function mediaSrc(key: MediaKey): string {
  const file = mediaRegistry[key].file;
  const asset = files[`../../assets/${file}`];
  if (!asset) throw new Error(`Missing local media asset: ${key}`);
  return asset;
}

/** Small functional icons and preview thumbnails retain their original XML catalog keys. */
export function designAsset(page: DesignPage, key: string): string {
  const asset = files[`../../assets/design/${page}-${key}.svg`];
  if (!asset) throw new Error(`Missing local design asset: ${page}/${key}`);
  return asset;
}
