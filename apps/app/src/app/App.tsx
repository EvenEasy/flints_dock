import { useCallback, useEffect, useRef, useState } from 'react';
import { forgetLocalWallet, readableError } from '../shared/api/wallet';
import { Notice } from '../shared/ui/Notice';
import type { AnalyzeWalletRequest, WalletConnection } from '../../frontend-contract/types';
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

/** Coordinates presentation, local identity, and read-only analysis; cleanup business logic belongs to src-tauri. */
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
  const hasSnapshot = useRef(false);
  const lastRequest = useRef<AnalyzeWalletRequest | null>(null);
  const lastScreen = useRef({ screen, preview });
  const [address, setAddress] = useState('');
  const [connection, setConnection] = useState<WalletConnection | null>(null);
  const [sessionError, setSessionError] = useState<string | null>(null);
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
        !preview && !hasSnapshot.current && next !== 'welcome' && next !== 'scanning'
          ? 'welcome'
          : next,
      );
      setDialog(null);
    }
    window.addEventListener('hashchange', onHashChange);
    return () => window.removeEventListener('hashchange', onHashChange);
  }, [preview]);

  // Move keyboard focus to the new screen heading after navigation, without announcing every rerender.
  useEffect(() => {
    // Preserve the initial tab order so keyboard users can reach the skip link first.
    if (lastScreen.current.screen !== screen || lastScreen.current.preview !== preview) {
      document.querySelector<HTMLElement>('#main-content h1')?.focus({ preventScroll: true });
      lastScreen.current = { screen, preview };
    }
  }, [screen, preview]);

  async function beginPreview() {
    if (connection) {
      try {
        await forgetLocalWallet();
      } catch (failure: unknown) {
        setSessionError(readableError(failure).message);
        return;
      }
      setConnection(null);
    }
    activeRequest.current += 1;
    hasSnapshot.current = false;
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
    hasSnapshot.current = false;
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
    hasSnapshot.current = false;
    lastRequest.current = request;
    setAddress(request.walletAddress);
    setIgnoredMints(new Set());
    navigate('scanning');
    const complete = await scan(request);
    // A dismissed request must not reopen its result screen when its RPC response arrives later.
    if (current === activeRequest.current && window.location.hash === '#scanning' && complete) {
      // Hash events can arrive before React commits the snapshot to the rendered view.
      hasSnapshot.current = true;
      navigate('summary');
    }
  }

  async function disconnect() {
    setDialog(null);
    try {
      await forgetLocalWallet();
    } catch (failure: unknown) {
      setSessionError(readableError(failure).message);
      return;
    }
    activeRequest.current += 1;
    hasSnapshot.current = false;
    lastRequest.current = null;
    reset();
    setConnection(null);
    setSessionError(null);
    setAddress('');
    setIgnoredMints(new Set());
    navigate('welcome');
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
    scanning: (
      <ScanScreen
        {...props}
        error={error}
        onRetry={() => {
          if (lastRequest.current) void startScan(lastRequest.current);
        }}
        onReconnect={() => setDialog('connect')}
      />
    ),
    summary: <SummaryScreen {...props} connection={connection} />,
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
          hasSnapshot.current = false;
          reset();
        }
        navigate(backScreen[screen]);
      }}
      onMenu={() => setDialog('menu')}
    >
      {sessionError && (
        <Notice tone="red" alert title="WALLET SESSION ERROR">
          {sessionError}
        </Notice>
      )}
      {content}
      {dialog === 'connect' && (
        <ConnectWalletDialog
          initialAddress={address}
          onConnected={(wallet) => {
            setConnection(wallet);
            setAddress(wallet.walletAddress);
            setSessionError(null);
          }}
          onPreview={beginPreview}
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
          connected={connection !== null}
          onDisconnect={() => {
            void disconnect();
          }}
          onClose={() => setDialog(null)}
          onNavigate={navigate}
          onConnect={() => setDialog('connect')}
          onPreview={beginPreview}
          onExitPreview={exitPreview}
        />
      )}
    </AppShell>
  );
}
