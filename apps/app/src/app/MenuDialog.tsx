import { Dialog } from '../shared/ui/Dialog';
import { screens } from './navigation';
import type { ScreenId } from './navigation';

/** Only reference mode exposes the simulated execution screens as navigation destinations. */
export function MenuDialog({
  preview,
  hasAnalysis,
  screen,
  onClose,
  onNavigate,
  onConnect,
  onPreview,
  onExitPreview,
  connected,
  onDisconnect,
}: {
  preview: boolean;
  hasAnalysis: boolean;
  screen: ScreenId;
  onClose: () => void;
  onNavigate: (screen: ScreenId) => void;
  onConnect: () => void;
  onPreview: () => void;
  onExitPreview: () => void;
  connected: boolean;
  onDisconnect: () => void;
}) {
  const destinations = preview
    ? screens
    : screens.filter(
        (item) =>
          item.id === 'welcome' ||
          (hasAnalysis && ['summary', 'tokens', 'dead', 'nfts', 'manifest'].includes(item.id)),
      );
  return (
    <Dialog title="STATION NAVIGATION" onClose={onClose}>
      <p className="dialog-copy">
        {preview
          ? 'Browse all ten reference screens. These values and operations are simulated.'
          : 'Analyze a public address in the desktop app. Cleanup execution is not available yet.'}
      </p>
      <nav className="screen-navigation" aria-label="Station screens">
        {destinations.map((item) => (
          <button
            key={item.id}
            type="button"
            aria-current={item.id === screen ? 'page' : undefined}
            onClick={() => onNavigate(item.id)}
          >
            {item.label}
            <span aria-hidden="true">↗</span>
          </button>
        ))}
      </nav>
      {preview && (
        <button type="button" className="text-button" onClick={onExitPreview}>
          EXIT PREVIEW
        </button>
      )}
      {connected && (
        <button type="button" className="text-button" onClick={onDisconnect}>
          DISCONNECT WALLET
        </button>
      )}
      {!preview && (
        <>
          <button type="button" className="text-button" onClick={onConnect}>
            CHANGE WALLET / RESCAN
          </button>
          <button type="button" className="text-button" onClick={onPreview}>
            EXPLORE DESIGN PREVIEW
          </button>
        </>
      )}
    </Dialog>
  );
}
