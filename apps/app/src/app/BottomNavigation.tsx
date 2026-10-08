import { designAsset } from '../shared/assets';
import { SurfaceFrame } from '../shared/ui/Design';
import type { DockSection } from './navigation';

const sections = [
  ['station', 'СТАНЦІЯ', 'nav_home_icon'],
  ['hangar', 'АНГАР', 'nav_rocket_icon'],
  ['holds', 'ТРЮМИ', 'nav_cargo_icon'],
  ['missions', 'МІСІЇ', 'nav_target_icon'],
  ['profile', 'ПРОФІЛЬ', 'nav_skull_icon'],
] as const;

/** Keep the XML's five sections, ordering, and icons across every connected screen. */
export function BottomNavigation({
  onNavigate,
  main = false,
}: {
  onNavigate: (section: DockSection) => void;
  main?: boolean;
}) {
  return (
    <nav className="bottom-navigation" aria-label="Основна навігація">
      <SurfaceFrame
        page={main ? 'main' : 'cleanup'}
        asset={main ? 'frame_navigation' : 'navigation_shell'}
      />
      {sections.map(([key, label, icon]) => (
        <button
          key={key}
          type="button"
          aria-current={key === 'station' ? 'page' : undefined}
          onClick={() => onNavigate(key)}
        >
          {key === 'station' && (
            <SurfaceFrame
              page={main ? 'main' : 'cleanup'}
              asset={main ? 'frame_nav_selected' : 'navigation_selected_shell'}
            />
          )}
          <img src={designAsset('cleanup', icon)} alt="" aria-hidden="true" />
          <span>{label}</span>
        </button>
      ))}
    </nav>
  );
}
