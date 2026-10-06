import { useCallback, useEffect, useRef, useState } from 'react';
import type { AnalyzeWalletRequest } from '../../frontend-contract/types';
import { AppShell } from './AppShell';
import { MenuDialog } from './MenuDialog';
import { backScreen, screenFromHash } from './navigation';
import type { ScreenId } from './navigation';
import { WelcomeScreen } from '../features/welcome/WelcomeScreen';
import { ConnectWalletDialog } from '../features/wallet/ConnectWalletDialog';
import { useWalletAnalysis } from '../features/wallet/useWalletAnalysis';
import { ScanScreen } from '../features/wallet/ScanScreen';
import { SummaryScreen } from '../features/wallet/SummaryScreen';
import { TokensScreen } from '../features/assets/TokensScreen';
import { DeadTokensScreen } from '../features/assets/DeadTokensScreen';
import { NftsScreen } from '../features/assets/NftsScreen';
import { ManifestScreen } from '../features/cleanup/ManifestScreen';
import { ConfirmScreen } from '../features/cleanup/ConfirmScreen';
import { SalvageScreen } from '../features/cleanup/SalvageScreen';
import { SuccessScreen } from '../features/cleanup/SuccessScreen';

/** Coordinates presentation and read-only IPC; cleanup business logic belongs to src-tauri. */
export function App() {
  const [preview, setPreview] = useState(
    () => new URLSearchParams(window.location.search).get('preview') === '1',
  );
  const [screen, setScreen] = useState<ScreenId>(() =>
    new URLSearchParams(window.location.search).get('preview') === '1'
      ? screenFromHash()
      : 'welcome',
  );
  const [dialog, setDialog] = useState<'connect' | 'menu' | null>(null);
  const activeRequest = useRef(0);
  const lastScreen = useRef({ screen, preview });
  const [address, setAddress] = useState('');
  const [ignoredMints, setIgnoredMints] = useState<ReadonlySet<string>>(new Set());
  const { analysis, error, scan, reset } = useWalletAnalysis();

  const navigate = useCallback((next: ScreenId) => {
    setScreen(next);
    setDialog(null);
    if (window.location.hash !== `#${next}`) window.location.hash = next;
  }, []);

  // Hash navigation is allowlisted; a live session cannot manufacture a wallet result from a URL.
  useEffect(() => {
    function onHashChange() {
      const next = screenFromHash();
      setScreen(
        !preview && !analysis && next !== 'welcome' && next !== 'scanning' ? 'welcome' : next,
      );
      setDialog(null);
    }
    window.addEventListener('hashchange', onHashChange);
    return () => window.removeEventListener('hashchange', onHashChange);
  }, [preview, analysis]);

  // Move keyboard focus to the new screen heading after navigation, without announcing every rerender.
  useEffect(() => {
    // Preserve the initial tab order so keyboard users can reach the skip link first.
    if (lastScreen.current.screen !== screen || lastScreen.current.preview !== preview) {
      document.querySelector<HTMLElement>('#main-content h1')?.focus({ preventScroll: true });
      lastScreen.current = { screen, preview };
    }
  }, [screen, preview]);

  function beginPreview() {
    activeRequest.current += 1;
    reset();
    setIgnoredMints(new Set());
    setPreview(true);
    const url = new URL(window.location.href);
    url.searchParams.set('preview', '1');
    window.history.replaceState(null, '', url);
    navigate('welcome');
  }

  function exitPreview() {
    activeRequest.current += 1;
    reset();
    setPreview(false);
    setIgnoredMints(new Set());
    const url = new URL(window.location.href);
    url.searchParams.delete('preview');
    window.history.replaceState(null, '', url);
    navigate('welcome');
  }

  async function startScan(request: AnalyzeWalletRequest) {
    const current = ++activeRequest.current;
    setAddress(request.walletAddress);
    setIgnoredMints(new Set());
    navigate('scanning');
    const complete = await scan(request);
    // A dismissed request must not reopen its result screen when its RPC response arrives later.
    if (current === activeRequest.current && window.location.hash === '#scanning')
      navigate(complete ? 'summary' : 'welcome');
  }

  function toggleMint(mint: string) {
    setIgnoredMints((previous) => {
      const next = new Set(previous);
      if (next.has(mint)) next.delete(mint);
      else next.add(mint);
      return next;
    });
  }

  const props = { preview, analysis, onNavigate: navigate };
  const content = {
    welcome: (
      <WelcomeScreen
        preview={preview}
        error={error}
        onConnect={() => (preview ? navigate('scanning') : setDialog('connect'))}
        onPreview={beginPreview}
      />
    ),
    scanning: <ScanScreen {...props} />,
    summary: <SummaryScreen {...props} />,
    tokens: <TokensScreen {...props} ignoredMints={ignoredMints} onToggleMint={toggleMint} />,
    dead: <DeadTokensScreen {...props} />,
    nfts: <NftsScreen {...props} />,
    manifest: <ManifestScreen {...props} />,
    confirm: <ConfirmScreen {...props} />,
    salvage: <SalvageScreen {...props} />,
    success: <SuccessScreen {...props} />,
  }[screen];

  return (
    <AppShell
      screen={screen}
      preview={preview}
      onBack={() => {
        if (screen === 'scanning' && !preview) {
          activeRequest.current += 1;
          reset();
        }
        navigate(backScreen[screen]);
      }}
      onMenu={() => setDialog('menu')}
      onExitPreview={exitPreview}
    >
      {content}
      {dialog === 'connect' && (
        <ConnectWalletDialog
          initialAddress={address}
          onClose={() => setDialog(null)}
          onScan={(request) => {
            void startScan(request);
          }}
        />
      )}
      {dialog === 'menu' && (
        <MenuDialog
          screen={screen}
          preview={preview}
          hasAnalysis={analysis !== null}
          onClose={() => setDialog(null)}
          onNavigate={navigate}
          onConnect={() => setDialog('connect')}
          onPreview={beginPreview}
        />
      )}
    </AppShell>
  );
}
