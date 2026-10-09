export const screens = [
  { id: 'welcome', label: 'Connection' },
  { id: 'scanning', label: 'Wallet analysis' },
  { id: 'main', label: 'Station' },
  { id: 'cleanup', label: 'Cleanup' },
  { id: 'success', label: 'Cleanup complete' },
] as const;
export type ScreenId = (typeof screens)[number]['id'] | 'processing';
export type DockSection = 'station' | 'hangar' | 'holds' | 'missions' | 'profile';

/** Hashes select known screens only; live result access is additionally guarded by App. */
export function screenFromHash(): ScreenId {
  if (window.location.hash === '#processing') return 'processing';
  return screens.find((screen) => screen.id === window.location.hash.slice(1))?.id ?? 'welcome';
}
