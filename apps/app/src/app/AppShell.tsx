import { useState } from 'react';
import type { CSSProperties, ReactNode } from 'react';
import { SceneBackground } from '../shared/ui/Design';
import { displayOptions, PHONE_ASPECT_RATIO, PHONE_MIN_WIDTH } from './display';
import { BottomNavigation } from './BottomNavigation';
import type { DockSection, ScreenId } from './navigation';

/** Fit the UI into a portrait phone viewport, keeping lists and navigation in separate rows. */
export function AppShell({
  screen,
  preview,
  children,
  onSection,
  onPreviewMenu,
}: {
  screen: ScreenId;
  preview: boolean;
  children: ReactNode;
  onSection: (section: DockSection) => void;
  onPreviewMenu: () => void;
}) {
  const [display] = useState(() =>
    displayOptions(window.location.search, {
      width: import.meta.env.VITE_DOCK_WIDTH,
      backdrop: import.meta.env.VITE_DOCK_BACKDROP,
    }),
  );
  const reference =
    preview && new URLSearchParams(window.location.search).get('layout') === 'reference';
  const connectedPage = ['main', 'cleanup', 'processing', 'success'].includes(screen);
  return (
    <div
      className={`app-viewport ${display.backdrop ? 'app-backdrop' : ''} ${reference ? 'reference-mode' : 'phone-layout'}`}
      style={
        {
          '--screen-width': `${reference ? (screen === 'main' ? 853 : 1024) : display.width}px`,
          '--phone-aspect-ratio': PHONE_ASPECT_RATIO,
          '--phone-min-width': `${PHONE_MIN_WIDTH}px`,
        } as CSSProperties
      }
    >
      <a
        className="skip-link"
        href="#main-content"
        onClick={(event) => {
          event.preventDefault();
          document.getElementById('main-content')?.focus();
        }}
      >
        Skip to content
      </a>
      {preview && (
        <button type="button" className="preview-indicator" onClick={onPreviewMenu}>
          DESIGN PREVIEW · NO TRANSACTIONS
        </button>
      )}
      <section aria-label="Wallet app screen" className={`dock-screen dock-screen--${screen}`}>
        <SceneBackground />

        <main
          id="main-content"
          tabIndex={-1}
          aria-labelledby="screen-heading"
          className="screen-content"
          key={`${preview}:${screen}`}
        >
          {children}
        </main>
        {connectedPage && <BottomNavigation active="station" onNavigate={onSection} />}
      </section>
    </div>
  );
}
