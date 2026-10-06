export const screens = [
  { id: 'welcome', number: '01', label: 'Welcome', frame: 's01' },
  { id: 'scanning', number: '02', label: 'Scanning wallet', frame: 's02' },
  { id: 'summary', number: '03', label: 'Scan complete', frame: 's03' },
  { id: 'tokens', number: '04', label: 'Swappable tokens', frame: 's04' },
  { id: 'dead', number: '05', label: 'Dead tokens', frame: 's05' },
  { id: 'nfts', number: '06', label: 'NFTs', frame: 's06' },
  { id: 'manifest', number: '07', label: 'Cleanup manifest', frame: 's07' },
  { id: 'confirm', number: '08', label: 'Confirm cleanup', frame: 's08' },
  { id: 'salvage', number: '09', label: 'Salvage operation', frame: 's09' },
  { id: 'success', number: '10', label: 'Cargo hold clean', frame: 's10' },
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
