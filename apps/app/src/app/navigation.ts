export const screens = [
  { id: 'welcome', label: 'Welcome', frame: 's01' },
  { id: 'scanning', label: 'Scanning wallet', frame: 's02' },
  { id: 'summary', label: 'Scan complete', frame: 's03' },
  { id: 'tokens', label: 'Swappable tokens', frame: 's04' },
  { id: 'dead', label: 'Dead tokens', frame: 's05' },
  { id: 'nfts', label: 'NFTs', frame: 's06' },
  { id: 'manifest', label: 'Cleanup manifest', frame: 's07' },
  { id: 'confirm', label: 'Confirm cleanup', frame: 's08' },
  { id: 'salvage', label: 'Salvage operation', frame: 's09' },
  { id: 'success', label: 'Cargo hold clean', frame: 's10' },
] as const;

export type ScreenId = (typeof screens)[number]['id'];

export function screenFromHash(): ScreenId {
  const id = window.location.hash.slice(1);
  return screens.find((screen) => screen.id === id)?.id ?? 'welcome';
}

export const backScreen: Record<ScreenId, ScreenId> = {
  welcome: 'welcome',
  scanning: 'welcome',
  summary: 'welcome',
  tokens: 'manifest',
  dead: 'manifest',
  nfts: 'manifest',
  manifest: 'summary',
  confirm: 'manifest',
  salvage: 'confirm',
  success: 'summary',
};
