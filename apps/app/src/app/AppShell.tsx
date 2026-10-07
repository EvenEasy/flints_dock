import { useState } from 'react';
import type { CSSProperties, ReactNode } from 'react';
import { media } from '../shared/assets';
import { Icon } from '../shared/ui/Icon';
import { screens } from './navigation';
import type { ScreenId } from './navigation';
import { displayOptions } from './display';

/** A centered, bounded phone viewport scrolls content without stretching its metallic frame. */
export function AppShell({
  screen,
  preview,
  children,
  onBack,
  onMenu,
}: {
  screen: ScreenId;
  preview: boolean;
  children: ReactNode;
  onBack: () => void;
  onMenu: () => void;
}) {
  const [display] = useState(() =>
    displayOptions(window.location.search, {
      width: import.meta.env.VITE_DOCK_WIDTH,
      backdrop: import.meta.env.VITE_DOCK_BACKDROP,
    }),
  );
  const descriptor = screens.find((item) => item.id === screen)!;
  return (
    <div
      className={`app-viewport ${display.backdrop ? 'app-backdrop stars' : ''}`}
      style={{ '--phone-width': `${display.width}px` } as CSSProperties}
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
      <section aria-label="Wallet app screen" className={`station-shell station-shell--${screen}`}>
        <img
          className="station-frame"
          src={media(`frames/frame-${descriptor.frame}.png`)}
          alt=""
          aria-hidden="true"
        />
        {preview && <div className="preview-indicator">REFERENCE PREVIEW · NO TRANSACTIONS</div>}
        <header className="station-header">
          <button
            className="icon-button"
            type="button"
            aria-label={screen === 'scanning' && !preview ? 'Dismiss scan view' : 'Go back'}
            disabled={screen === 'welcome'}
            onClick={onBack}
          >
            <Icon name="back" />
          </button>
          <div className="station-wordmark">
            <img src={media('brand/pirate-emblem.png')} alt="" width="31" height="26" />
            <span>FLINT’S DOCK</span>
          </div>
          <button
            className="icon-button"
            type="button"
            aria-label="Open navigation"
            onClick={onMenu}
          >
            <Icon name="menu" />
          </button>
        </header>
        <main
          id="main-content"
          tabIndex={-1}
          className="screen-content"
          key={`${preview}:${screen}`}
        >
          {children}
        </main>
      </section>
    </div>
  );
}
