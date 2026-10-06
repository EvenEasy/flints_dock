import type { ReactNode } from 'react';
import { media } from '../shared/assets';
import { Icon } from '../shared/ui/Icon';
import { screens } from './navigation';
import type { ScreenId } from './navigation';

/** A responsive station frame keeps the portrait composition without drawing a fake phone status bar. */
export function AppShell({
  screen,
  preview,
  children,
  onBack,
  onMenu,
  onExitPreview,
}: {
  screen: ScreenId;
  preview: boolean;
  children: ReactNode;
  onBack: () => void;
  onMenu: () => void;
  onExitPreview: () => void;
}) {
  const descriptor = screens.find((item) => item.id === screen)!;
  return (
    <div className="app-backdrop stars">
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
      <div className="app-context">
        <span className={`mode-badge ${preview ? 'mode-badge--preview' : ''}`}>
          {preview ? 'REFERENCE PREVIEW · NO TRANSACTIONS' : 'DESKTOP WALLET ANALYSIS · READ ONLY'}
        </span>
        {preview && (
          <button type="button" onClick={onExitPreview}>
            EXIT PREVIEW
          </button>
        )}
      </div>
      <div className={`station-shell station-shell--${screen}`}>
        <img
          className="station-frame"
          src={media(`frames/frame-${descriptor.frame}.png`)}
          alt=""
          aria-hidden="true"
        />
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
        <footer className="station-footer">
          <span>ORBITAL SALVAGE TERMINAL</span>
          <span>{descriptor.number} / 10</span>
        </footer>
      </div>
      <p className="app-caption">
        FLINT’S DOCK <span>✦</span> SOLANA CARGO CONTROL
      </p>
    </div>
  );
}
