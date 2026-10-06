// Keep media references local and explicit; wallet metadata never chooses an image URL.
const files = import.meta.glob<string>(
  [
    '../../assets/brand/*.png',
    '../../assets/illustrations/*.png',
    '../../assets/frames/*.png',
    '../../assets/nfts/*.png',
    '../../assets/tokens/*.png',
    '../../assets/icons/base/*.png',
  ],
  { eager: true, query: '?url', import: 'default' },
);

export function media(path: string): string {
  const asset = files[`../../assets/${path}`];
  if (!asset) throw new Error('A bundled interface asset is missing');
  return asset;
}

export type IconName =
  | 'back'
  | 'menu'
  | 'wallet'
  | 'skull'
  | 'trash'
  | 'coins'
  | 'nft'
  | 'swap'
  | 'accounts'
  | 'fire'
  | 'shield'
  | 'warning'
  | 'check'
  | 'check-circle'
  | 'spinner'
  | 'pending'
  | 'chevron'
  | 'chevrons'
  | 'info'
  | 'boat'
  | 'document'
  | 'raptor-action'
  | 'salvage-coins';
export type Tone = 'cyan' | 'purple' | 'green' | 'red' | 'amber' | 'muted';
