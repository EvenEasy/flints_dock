export const screens = [
  { id: 'welcome', label: 'Підключення' },
  { id: 'scanning', label: 'Аналіз гаманця' },
  { id: 'main', label: 'Станція' },
  { id: 'cleanup', label: 'Очищення' },
  { id: 'success', label: 'Очищення завершено' },
] as const;
export type ScreenId = (typeof screens)[number]['id'] | 'processing';
export type DockSection = 'station' | 'hangar' | 'holds' | 'missions' | 'profile';

/** Hashes select known screens only; live result access is additionally guarded by App. */
export function screenFromHash(): ScreenId {
  if (window.location.hash === '#processing') return 'processing';
  return screens.find((screen) => screen.id === window.location.hash.slice(1))?.id ?? 'welcome';
}
