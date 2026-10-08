import { useCallback, useEffect, useRef, useState } from 'react';
import { forgetLocalWallet, readableError } from '../shared/api/wallet';
import { Notice } from '../shared/ui/Notice';
import { Dialog } from '../shared/ui/Dialog';
import type { AnalyzeWalletRequest, WalletConnection } from '../../frontend-contract/types';
import { AppShell } from './AppShell';
import { MenuDialog } from './MenuDialog';
import { screenFromHash } from './navigation';
import type { DockSection, ScreenId } from './navigation';
import { WelcomeScreen } from '../features/welcome/WelcomeScreen';
import { ConnectWalletDialog } from '../features/wallet/ConnectWalletDialog';
import { useWalletAnalysis } from '../features/wallet/useWalletAnalysis';
import { ScanScreen } from '../features/wallet/ScanScreen';
import { MainScreen } from '../features/wallet/MainScreen';
import type { CargoCategory } from '../features/wallet/MainScreen';
import { CleanupScreen } from '../features/cleanup/CleanupScreen';
import { SuccessScreen } from '../features/cleanup/SuccessScreen';
import { InventoryDialog } from '../features/assets/InventoryDialog';
import { previewExcluded } from '../features/preview/designData';

/** Frontend orchestration owns navigation and selection; Rust owns credentials, RPC, and calculations. */
export function App() {
  const [preview, setPreview] = useState(
    () => new URLSearchParams(window.location.search).get('preview') === '1',
  );
  const [screen, setScreen] = useState<ScreenId>(() =>
    new URLSearchParams(window.location.search).get('preview') === '1'
      ? screenFromHash()
      : 'welcome',
  );
  const [dialog, setDialog] = useState<'connect' | 'menu' | 'inventory' | 'capability' | null>(
    null,
  );
  const [inventoryView, setInventoryView] = useState<'tokens' | 'nfts'>('tokens');
  const [capability, setCapability] = useState('');
  const activeRequest = useRef(0);
  const hasSnapshot = useRef(false);
  const hasScanRequest = useRef(false);
  const lastRequest = useRef<AnalyzeWalletRequest | null>(null);
  const lastScreen = useRef({ screen, preview });
  const [address, setAddress] = useState('');
  const [connection, setConnection] = useState<WalletConnection | null>(null);
  const [sessionError, setSessionError] = useState<string | null>(null);
  const [ignoredMints, setIgnoredMints] = useState<ReadonlySet<string>>(
    () => new Set(preview ? previewExcluded : []),
  );
  const { analysis, error, scan, reset } = useWalletAnalysis();

  const navigate = useCallback(
    (next: ScreenId) => {
      // A URL or navigation callback cannot create a successful on-chain cleanup result.
      const allowed = next === 'success' && !preview ? 'main' : next;
      setScreen(allowed);
      setDialog(null);
      if (window.location.hash !== `#${allowed}`) window.location.hash = allowed;
    },
    [preview],
  );

  // Late hash events cannot bounce an accepted snapshot back to the connection screen.
  useEffect(() => {
    function onHashChange() {
      const next = screenFromHash();
      // Navigating away dismisses pending presentation; the backend RPC itself has no cancellation API.
      if (!preview && next === 'welcome' && hasScanRequest.current) {
        activeRequest.current += 1;
        hasScanRequest.current = false;
        hasSnapshot.current = false;
        reset();
      }
      let allowed = next;
      if (!preview) {
        if (next === 'scanning' && !hasScanRequest.current)
          allowed = hasSnapshot.current ? 'main' : 'welcome';
        else if (next === 'success') allowed = hasSnapshot.current ? 'main' : 'welcome';
        else if (!hasSnapshot.current && next !== 'welcome' && next !== 'scanning')
          allowed = 'welcome';
      }
      setScreen(allowed);
      if (allowed !== next) {
        const url = new URL(window.location.href);
        url.hash = allowed;
        window.history.replaceState(null, '', url);
      }
      setDialog(null);
    }
    window.addEventListener('hashchange', onHashChange);
    return () => window.removeEventListener('hashchange', onHashChange);
  }, [preview, reset]);

  // Preserve the initial skip-link tab order, then focus headings after screen changes.
  useEffect(() => {
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
    hasScanRequest.current = false;
    lastRequest.current = null;
    reset();
    setAddress('');
    setSessionError(null);
    setIgnoredMints(new Set(previewExcluded));
    setPreview(true);
    const url = new URL(window.location.href);
    url.searchParams.set('preview', '1');
    window.history.replaceState(null, '', url);
    navigate('welcome');
  }

  function exitPreview() {
    activeRequest.current += 1;
    hasSnapshot.current = false;
    hasScanRequest.current = false;
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
    hasScanRequest.current = true;
    lastRequest.current = request;
    setAddress(request.walletAddress);
    setIgnoredMints(new Set());
    navigate('scanning');
    const complete = await scan(request);
    // Dismissed or superseded RPC work must not reopen its eventual result.
    if (current === activeRequest.current && window.location.hash === '#scanning' && complete) {
      hasSnapshot.current = true;
      hasScanRequest.current = false;
      navigate('main');
    }
  }

  function dismissScan() {
    activeRequest.current += 1;
    hasSnapshot.current = false;
    hasScanRequest.current = false;
    reset();
    navigate('welcome');
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
    hasScanRequest.current = false;
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

  function onSection(section: DockSection) {
    if (section === 'station') navigate('main');
    else if (section === 'profile') setDialog('menu');
    else if (section === 'holds') {
      setInventoryView('tokens');
      setDialog('inventory');
    } else {
      setCapability(
        section === 'hangar'
          ? 'Swap routes and execution are not exposed by the desktop API.'
          : 'Mission data is not exposed by the desktop API.',
      );
      setDialog('capability');
    }
  }

  function inspectCategory(category: CargoCategory) {
    if (category === 'nft') {
      setInventoryView('nfts');
      setDialog('inventory');
    } else {
      setCapability(
        preview
          ? 'Reference category counts are design samples. Inspect the cleanup preview to explore sample assets.'
          : 'Risk, dust, and dead-token classifications are not supplied by this snapshot. Missing Jupiter USD prices do not establish that a token is dead or unswappable.',
      );
      setDialog('capability');
    }
  }

  const content = {
    welcome: (
      <WelcomeScreen onConnect={() => (preview ? navigate('scanning') : setDialog('connect'))} />
    ),
    scanning: (
      <ScanScreen
        preview={preview}
        error={error}
        onRetry={() => {
          if (lastRequest.current) void startScan(lastRequest.current);
        }}
        onReconnect={() => setDialog('connect')}
        onDismiss={dismissScan}
      />
    ),
    main: (
      <MainScreen
        preview={preview}
        analysis={analysis}
        onCleanup={() => navigate('cleanup')}
        onInspect={inspectCategory}
      />
    ),
    cleanup: (
      <CleanupScreen
        preview={preview}
        analysis={analysis}
        ignoredMints={ignoredMints}
        onToggleMint={toggleMint}
        onPreviewComplete={() => {
          if (preview) navigate('success');
        }}
      />
    ),
    success: <SuccessScreen displayAmount="+0.428 SOL" />,
  }[screen];

  return (
    <AppShell
      screen={screen}
      preview={preview}
      onSection={onSection}
      onPreviewMenu={() => setDialog('menu')}
    >
      {sessionError && (
        <Notice tone="red" alert title="WALLET SESSION ERROR">
          {sessionError}
        </Notice>
      )}
      {content}
      {screen === 'welcome' && connection && (
        <button
          type="button"
          className="session-shortcut text-button"
          onClick={() => setDialog('menu')}
        >
          CONNECTED WALLET
        </button>
      )}
      {dialog === 'connect' && (
        <ConnectWalletDialog
          initialAddress={address}
          onConnected={(wallet) => {
            setConnection(wallet);
            setAddress(wallet.walletAddress);
            setSessionError(null);
          }}
          onPreview={() => {
            void beginPreview();
          }}
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
          analysis={analysis}
          connection={connection}
          onDisconnect={() => {
            void disconnect();
          }}
          onClose={() => setDialog(null)}
          onNavigate={navigate}
          onConnect={() => setDialog('connect')}
          onPreview={() => {
            void beginPreview();
          }}
          onExitPreview={exitPreview}
        />
      )}
      {dialog === 'inventory' && (
        <InventoryDialog
          analysis={preview ? null : analysis}
          initialView={inventoryView}
          onClose={() => setDialog(null)}
        />
      )}
      {dialog === 'capability' && (
        <Dialog title="МОЖЛИВІСТЬ НЕДОСТУПНА" onClose={() => setDialog(null)}>
          <p>{capability}</p>
        </Dialog>
      )}
    </AppShell>
  );
}
