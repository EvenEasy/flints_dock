import { mediaSrc } from '../shared/assets';
import type { MediaKey } from '../shared/assets';
import { SurfaceFrame } from '../shared/ui/Design';
import type { DockSection } from './navigation';

const sections: readonly [DockSection, string, MediaKey][] = [
  ['station', 'СТАНЦІЯ', 'navigation.station'],
  ['hangar', 'АНГАР', 'navigation.hangar'],
  ['holds', 'ТРЮМИ', 'navigation.holds'],
  ['missions', 'МІСІЇ', 'navigation.missions'],
  ['profile', 'ПРОФІЛЬ', 'navigation.profile'],
];

/** Content and geometry are shared; callers explicitly choose the active section. */
export function BottomNavigation({
  onNavigate,
  active,
}: {
  onNavigate: (section: DockSection) => void;
  active: DockSection;
}) {
  return (
    <nav className="bottom-navigation" aria-label="Основна навігація">
      <SurfaceFrame />
      {sections.map(([key, label, icon]) => (
        <button
          key={key}
          type="button"
          aria-current={key === active ? 'page' : undefined}
          onClick={() => onNavigate(key)}
        >
          {key === active && <SurfaceFrame className="navigation-selected" />}
          <img src={mediaSrc(icon)} alt="" aria-hidden="true" />
          <span className="type-navigation">{label}</span>
        </button>
      ))}
    </nav>
  );
}
