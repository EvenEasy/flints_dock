import { act, render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { mockIPC } from '@tauri-apps/api/mocks';
import { App } from './App';
import { screens } from './navigation';
import { address, secondMint, token, wallet, pricedWallet } from '../test/fixtures';
import type { WalletAnalysis } from '../../frontend-contract/types';

function desktop(handler: Parameters<typeof mockIPC>[0]) {
  Reflect.set(globalThis, 'isTauri', true);
  mockIPC((command, payload) => {
    if (command === 'connect_wallet') {
      const request = (payload as { request: { source: { kind: string; address?: string } } })
        .request;
      return {
        walletAddress: request.source.address ?? address,
        sourceKind: request.source.kind,
        canSign: request.source.kind !== 'publicKey',
      };
    }
    if (command === 'disconnect_wallet') return null;
    return handler(command, payload);
  });
}

async function connect(user: ReturnType<typeof userEvent.setup>) {
  await user.click(screen.getByRole('button', { name: 'CONNECT WALLET' }));
  await user.click(screen.getByRole('radio', { name: 'Public key (read only)' }));
  await user.type(screen.getByRole('textbox', { name: 'WALLET PUBLIC KEY' }), address);
  await user.click(screen.getByRole('button', { name: 'SCAN WALLET' }));
}

async function go(user: ReturnType<typeof userEvent.setup>, label: string) {
  await user.click(screen.getByRole('button', { name: 'Open navigation' }));
  await user.click(
    within(screen.getByRole('navigation', { name: 'Station screens' })).getByRole('button', {
      name: new RegExp(label),
    }),
  );
}

describe('Read-only wallet integration', () => {
  it('Scan_InvalidAddress_ShowsValidationWithoutIPC', async () => {
    // Arrange
    const ipc = vi.fn(() => wallet());
    desktop(ipc);
    const user = userEvent.setup();
    render(<App />);

    // Act
    await user.click(screen.getByRole('button', { name: 'CONNECT WALLET' }));
    await user.click(screen.getByRole('radio', { name: 'Public key (read only)' }));
    await user.type(screen.getByRole('textbox', { name: 'WALLET PUBLIC KEY' }), 'not-a-wallet');
    await user.click(screen.getByRole('button', { name: 'SCAN WALLET' }));

    // Assert
    expect(screen.getByRole('alert')).toHaveTextContent('valid Solana public key');
    expect(ipc).not.toHaveBeenCalled();
  });

  it('Scan_PublicAddress_UsesExistingContractAndExactBalances', async () => {
    const ipc = vi.fn(() => wallet());
    desktop(ipc);
    const user = userEvent.setup();
    render(<App />);

    await connect(user);

    expect(await screen.findByRole('heading', { name: 'SCAN COMPLETE' })).toBeVisible();
    expect(ipc).toHaveBeenCalledExactlyOnceWith('analyze_wallet', {
      request: {
        walletAddress: address,
        noPrices: false,
        selection: { balance: true, tokens: true, allTokens: true, nfts: true, cnfts: true },
      },
    });
    expect(screen.getByText('12.345678901 SOL')).toBeVisible();
    await go(user, 'Swappable tokens');
    expect(screen.getAllByText('900,719,925,474.099312345')).toHaveLength(2);
    expect(screen.getAllByText('PRICE UNAVAILABLE')).toHaveLength(2);
  });

  it('Connect_BrowserRuntime_ExplainsDesktopBeforeCollectingAddress', async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(screen.getByRole('button', { name: 'CONNECT WALLET' }));

    const dialog = screen.getByRole('dialog', { name: 'DOCK A WALLET' });
    expect(within(dialog).getByText('DESKTOP APP REQUIRED')).toBeVisible();
    expect(within(dialog).getByText('npm run desktop')).toBeVisible();
    expect(screen.queryByRole('textbox', { name: 'WALLET PUBLIC KEY' })).not.toBeInTheDocument();
    await user.click(within(dialog).getByRole('button', { name: 'EXPLORE DESIGN PREVIEW' }));
    expect(screen.getByText('REFERENCE PREVIEW · NO TRANSACTIONS')).toBeVisible();
  });

  it('Scan_BackendRejects_ShowsPlainContractErrorAndRetry', async () => {
    desktop(() =>
      Promise.reject({
        code: 'invalid_configuration',
        message: 'RPC configuration is missing.',
        details: { secret: 'do-not-display' },
      }),
    );
    const user = userEvent.setup();
    render(<App />);

    await connect(user);

    expect(await screen.findByRole('alert')).toHaveTextContent('RPC configuration is missing.');
    expect(screen.queryByText('do-not-display')).not.toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'SCAN INTERRUPTED' })).toBeVisible();
    await user.click(screen.getByRole('button', { name: 'CHANGE WALLET' }));
    await user.click(screen.getByRole('radio', { name: 'Public key (read only)' }));
    expect(screen.getByRole('textbox', { name: 'WALLET PUBLIC KEY' })).toHaveValue(address);
  });

  it('Keep_SameSymbolDifferentMints_ProtectsOnlySelectedMint', async () => {
    const ipc = vi.fn(() => wallet());
    desktop(ipc);
    const user = userEvent.setup();
    render(<App />);
    await connect(user);
    await screen.findByRole('heading', { name: 'SCAN COMPLETE' });
    await go(user, 'Swappable tokens');

    await user.click(screen.getByRole('switch', { name: `Keep SAME mint ${address}` }));

    expect(screen.getByRole('switch', { name: `Keep SAME mint ${address}` })).toBeChecked();
    expect(screen.getByRole('switch', { name: `Keep SAME mint ${secondMint}` })).not.toBeChecked();
    expect(screen.getByText('1 MINTS MARKED TO KEEP')).toBeVisible();
    expect(ipc).toHaveBeenCalledTimes(1);
  });

  it('Inventory_UntrustedMetadata_RendersTextWithoutExternalArtwork', async () => {
    const asset = token();
    asset.metadata.symbol = '<img src=x onerror=alert(1)>';
    desktop(() =>
      wallet({
        tokens: {
          status: { status: 'partial', reason: 'Some metadata unavailable.' },
          items: [asset],
        },
      }),
    );
    const user = userEvent.setup();
    render(<App />);
    await connect(user);
    await screen.findByRole('heading', { name: 'SCAN COMPLETE' });

    await go(user, 'Swappable tokens');

    expect(screen.getByText('<img src=x onerror=alert(1)>')).toBeVisible();
    expect(screen.getByText('Some metadata unavailable.')).toBeVisible();
    expect(
      screen
        .getAllByRole('presentation', { hidden: true })
        .every((img) => !img.getAttribute('src')?.includes('untrusted.invalid')),
    ).toBe(true);
  });

  it.each([
    { items: [], message: 'No token assets found in this category.' },
    { items: null, message: 'Token inventory is unavailable. This is not an empty wallet.' },
  ])('Inventory_$message_PreservesUnavailableVersusEmpty', async ({ items, message }) => {
    desktop(() => wallet({ tokens: { status: { status: 'complete' }, items } }));
    const user = userEvent.setup();
    render(<App />);
    await connect(user);
    await screen.findByRole('heading', { name: 'SCAN COMPLETE' });

    await go(user, 'Swappable tokens');

    expect(screen.getByText(message)).toBeVisible();
  });

  it('Cleanup_LiveWallet_DisablesExecutionAndPreservesDiagnostics', async () => {
    const ipc = vi.fn(() => wallet());
    desktop(ipc);
    const user = userEvent.setup();
    render(<App />);
    await connect(user);
    await screen.findByRole('heading', { name: 'SCAN COMPLETE' });
    await user.click(screen.getByRole('button', { name: 'REVIEW CLEANUP' }));
    expect(screen.getByText('ACCOUNT INVENTORY & DIAGNOSTICS')).toBeVisible();

    await user.click(screen.getByRole('button', { name: 'REVIEW REQUIRED CAPABILITIES' }));

    expect(screen.getByRole('button', { name: 'CLEANUP API REQUIRED' })).toBeDisabled();
    expect(ipc).toHaveBeenCalledTimes(1);
  });

  it('Inventory_MoreThanFiftyAssets_PaginatesWithoutLosingKeepIntent', async () => {
    const assets = Array.from({ length: 51 }, (_, index) =>
      token(
        'A'.repeat(42) + '123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz'[index] + '1',
      ),
    );
    desktop(() => wallet({ tokens: { status: { status: 'complete' }, items: assets } }));
    const user = userEvent.setup();
    render(<App />);
    await connect(user);
    await screen.findByRole('heading', { name: 'SCAN COMPLETE' });
    await go(user, 'Swappable tokens');
    expect(screen.getAllByRole('switch')).toHaveLength(50);
    await user.click(screen.getAllByRole('switch')[0]!);

    await user.click(screen.getByRole('button', { name: 'NEXT' }));
    expect(screen.getAllByRole('switch')).toHaveLength(1);
    await user.click(screen.getByRole('button', { name: 'PREVIOUS' }));

    expect(screen.getAllByRole('switch')[0]).toBeChecked();
    expect(screen.getByText('1 MINTS MARKED TO KEEP')).toBeVisible();
  });

  it('Scan_DismissedRequest_DiscardsLateResult', async () => {
    let resolve!: (value: WalletAnalysis) => void;
    desktop(
      () =>
        new Promise<WalletAnalysis>((done) => {
          resolve = done;
        }),
    );
    const user = userEvent.setup();
    render(<App />);
    await connect(user);
    expect(screen.getByRole('progressbar')).not.toHaveAttribute('aria-valuenow');

    await user.click(screen.getByRole('button', { name: 'Dismiss scan view' }));
    await act(async () => {
      resolve(wallet());
    });

    expect(screen.getByRole('heading', { name: 'FLINT’S DOCK' })).toBeVisible();
    expect(screen.queryByRole('heading', { name: 'SCAN COMPLETE' })).not.toBeInTheDocument();
  });
});

describe('Host compatibility', () => {
  it('Scan_OlderTauriBridge_UsesExistingIPCWithoutRuntimeFlag', async () => {
    const ipc = vi.fn(() => wallet());
    mockIPC((command) =>
      command === 'connect_wallet'
        ? { walletAddress: address, sourceKind: 'publicKey', canSign: false }
        : ipc(),
    );
    const user = userEvent.setup();
    render(<App />);

    await connect(user);

    expect(await screen.findByRole('heading', { name: 'SCAN COMPLETE' })).toBeVisible();
    expect(ipc).toHaveBeenCalledTimes(1);
  });
});

describe('Reference preview', () => {
  it.each(screens)('Render_$id_ShowsReferenceScreenWithoutIPC', ({ id }) => {
    const ipc = vi.fn();
    desktop(ipc);
    window.history.replaceState(null, '', `/?preview=1#${id}`);

    render(<App />);

    expect(screen.getByRole('heading', { level: 1 })).toBeVisible();
    expect(screen.getByText('REFERENCE PREVIEW · NO TRANSACTIONS')).toBeVisible();
    expect(ipc).not.toHaveBeenCalled();
  });

  it('Preview_FullFlow_NeverSubmitsIPC', async () => {
    const ipc = vi.fn();
    desktop(ipc);
    const user = userEvent.setup();
    render(<App />);

    await user.click(screen.getByRole('button', { name: /EXPLORE DESIGN PREVIEW/ }));
    await user.click(screen.getByRole('button', { name: 'CONNECT WALLET' }));
    await user.click(screen.getByRole('button', { name: 'PREVIEW SCAN RESULTS' }));
    await user.click(screen.getByRole('button', { name: 'REVIEW CLEANUP' }));
    await user.click(screen.getByRole('button', { name: 'CLEAN WALLET' }));
    await user.click(screen.getByRole('button', { name: 'CONFIRM CLEANUP' }));
    await user.click(screen.getByRole('button', { name: 'PREVIEW COMPLETION' }));

    expect(screen.getByRole('heading', { name: 'CARGO HOLD CLEAN' })).toBeVisible();
    expect(ipc).not.toHaveBeenCalled();
  });
});

describe('Local wallet identity', () => {
  it('Connect_Base64Seed_TransfersOnceThenScansOnlyPublicAddress', async () => {
    const seed = 'AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=';
    const ipc = vi.fn((command: string) =>
      command === 'connect_wallet'
        ? { walletAddress: address, sourceKind: 'seed', canSign: true }
        : wallet(),
    );
    Reflect.set(globalThis, 'isTauri', true);
    mockIPC(ipc);
    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByRole('button', { name: 'CONNECT WALLET' }));
    await user.type(screen.getByLabelText('SEED BASE64'), seed);

    await user.click(screen.getByRole('button', { name: 'SCAN WALLET' }));

    expect(await screen.findByText('LOCAL SIGNER CONNECTED')).toBeVisible();
    expect(ipc.mock.calls[0]).toEqual([
      'connect_wallet',
      { request: { source: { kind: 'seed', base64: seed } } },
    ]);
    expect(ipc.mock.calls[1]).toEqual([
      'analyze_wallet',
      {
        request: {
          walletAddress: address,
          selection: { balance: true, tokens: true, allTokens: true, nfts: true, cnfts: true },
          noPrices: false,
        },
      },
    ]);
    expect(screen.queryByDisplayValue(seed)).not.toBeInTheDocument();
    expect(window.localStorage.length).toBe(0);
  });

  it('Connect_KeypairPath_SendsPathWithoutReadingFileInFrontend', async () => {
    const ipc = vi.fn((command: string) =>
      command === 'connect_wallet'
        ? { walletAddress: address, sourceKind: 'keypairFile', canSign: true }
        : wallet(),
    );
    Reflect.set(globalThis, 'isTauri', true);
    mockIPC(ipc);
    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByRole('button', { name: 'CONNECT WALLET' }));
    await user.click(screen.getByRole('radio', { name: 'File (keypair)' }));
    await user.type(
      screen.getByRole('textbox', { name: 'KEYPAIR FILE PATH' }),
      '/home/example/id.json',
    );

    await user.click(screen.getByRole('button', { name: 'SCAN WALLET' }));

    expect(await screen.findByText('LOCAL SIGNER CONNECTED')).toBeVisible();
    expect(ipc.mock.calls[0]).toEqual([
      'connect_wallet',
      { request: { source: { kind: 'keypairFile', path: '/home/example/id.json' } } },
    ]);
  });

  it('Connect_RejectedSeed_ClearsSecretAndKeepsConnectionDialog', async () => {
    const seed = 'AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=';
    Reflect.set(globalThis, 'isTauri', true);
    mockIPC(() =>
      Promise.reject({
        code: 'invalid_wallet_identity',
        message: 'Seed could not be loaded.',
        details: null,
      }),
    );
    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByRole('button', { name: 'CONNECT WALLET' }));
    await user.type(screen.getByLabelText('SEED BASE64'), seed);

    await user.click(screen.getByRole('button', { name: 'SCAN WALLET' }));

    expect(await screen.findByRole('alert')).toHaveTextContent('Seed could not be loaded.');
    expect(screen.getByLabelText('SEED BASE64')).toHaveValue('');
    expect(screen.getByRole('dialog')).toBeVisible();
  });

  it('Disconnect_ConnectedWallet_ForgetsBackendSignerAndReturnsToEntry', async () => {
    const ipc = vi.fn((command: string) =>
      command === 'connect_wallet'
        ? { walletAddress: address, sourceKind: 'publicKey', canSign: false }
        : command === 'disconnect_wallet'
          ? null
          : wallet(),
    );
    Reflect.set(globalThis, 'isTauri', true);
    mockIPC(ipc);
    const user = userEvent.setup();
    render(<App />);
    await connect(user);
    await screen.findByRole('heading', { name: 'SCAN COMPLETE' });
    await user.click(screen.getByRole('button', { name: 'Open navigation' }));

    await user.click(screen.getByRole('button', { name: 'DISCONNECT WALLET' }));

    expect(await screen.findByRole('heading', { name: 'FLINT’S DOCK' })).toBeVisible();
    expect(ipc.mock.calls.map((call) => call[0])).toEqual([
      'connect_wallet',
      'analyze_wallet',
      'disconnect_wallet',
    ]);
  });

  it('Scan_TransportFailure_KeepsErrorScreenAndRetriesWithoutCredentialReplay', async () => {
    const ipc = vi
      .fn()
      .mockRejectedValueOnce('analyze_wallet not allowed on this window')
      .mockResolvedValueOnce(wallet());
    desktop(ipc);
    const user = userEvent.setup();
    render(<App />);
    await connect(user);
    expect(await screen.findByRole('alert')).toHaveTextContent('analyze_wallet not allowed');
    expect(screen.getByRole('heading', { name: 'SCAN INTERRUPTED' })).toBeVisible();

    await user.click(screen.getByRole('button', { name: 'RETRY SCAN' }));

    expect(await screen.findByRole('heading', { name: 'SCAN COMPLETE' })).toBeVisible();
    expect(ipc).toHaveBeenCalledTimes(2);
  });

  it('Scan_CompletedSnapshot_SurvivesHashNavigationAndPartialResults', async () => {
    desktop(() =>
      wallet({
        hasUsableResults: false,
        scanners: { native_sol: { status: 'failed', reason: 'RPC timeout' } },
      }),
    );
    const user = userEvent.setup();
    render(<App />);
    await connect(user);
    await screen.findByRole('heading', { name: 'SCAN COMPLETE' });

    await act(async () => {
      window.dispatchEvent(new HashChangeEvent('hashchange'));
    });

    expect(screen.getByRole('heading', { name: 'SCAN COMPLETE' })).toBeVisible();
    expect(screen.getByRole('alert')).toHaveTextContent('No usable asset results');
  });
});

describe('Jupiter price presentation', () => {
  it('Scan_DefaultPricing_ShowsSOLAndMintUnitPricesWithBackendValues', async () => {
    // Arrange: a priced snapshot arrives through the actual Tauri contract boundary.
    const ipc = vi.fn(() => pricedWallet());
    desktop(ipc);
    const user = userEvent.setup();
    render(<App />);

    // Act: connect and inspect the native balance and both token views.
    await connect(user);

    // Assert: unit prices and holding values remain distinct, including tiny prices.
    expect(await screen.findByText('1 SOL = $120.50')).toBeVisible();
    expect(screen.getByText('≈ $1,487.65')).toBeVisible();
    await go(user, 'Swappable tokens');
    expect(screen.getByRole('columnheader', { name: 'PRICE / TOKEN' })).toBeVisible();
    expect(screen.getByText('$1.23E-10')).toBeVisible();
    expect(screen.getByText('$1.0022')).toBeVisible();
    expect(screen.getByText('$10.02')).toBeVisible();
    expect(screen.queryByText('NOT QUOTED')).not.toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'ALL ACCOUNTS' }));
    expect(screen.getByText('$1.23E-10')).toBeVisible();
    expect(ipc).toHaveBeenCalledTimes(1);
  });

  it('Scan_PricingOptOut_PreservesUnpricedInventory', async () => {
    // Arrange
    const ipc = vi.fn(() =>
      wallet({ scanners: { prices: { status: 'skipped', reason: 'Prices disabled by user.' } } }),
    );
    desktop(ipc);
    const user = userEvent.setup();
    render(<App />);

    // Act: prices are checked by default and can still be disabled before scanning.
    await user.click(screen.getByRole('button', { name: 'CONNECT WALLET' }));
    const prices = screen.getByRole('checkbox', { name: 'Include Jupiter USD prices' });
    expect(prices).toBeChecked();
    await user.click(prices);
    await user.click(screen.getByRole('radio', { name: 'Public key (read only)' }));
    await user.type(screen.getByRole('textbox', { name: 'WALLET PUBLIC KEY' }), address);
    await user.click(screen.getByRole('button', { name: 'SCAN WALLET' }));

    // Assert: balances remain visible and unknown prices are never fabricated.
    expect(await screen.findByText('12.345678901 SOL')).toBeVisible();
    expect(screen.getByText('SOL price unavailable')).toBeVisible();
    expect(ipc).toHaveBeenCalledWith('analyze_wallet', {
      request: expect.objectContaining({ noPrices: true }),
    });
    await go(user, 'Swappable tokens');
    expect(screen.getAllByText('PRICE UNAVAILABLE')).toHaveLength(2);
    expect(screen.getAllByText('900,719,925,474.099312345')).toHaveLength(2);
  });

  it('Scan_PartialPricing_ShowsAvailablePricesAndProviderDiagnostics', async () => {
    // Arrange: one missing quote does not erase a successful native/token quote.
    const snapshot = pricedWallet();
    snapshot.tokens!.items![1]!.price = null;
    snapshot.tokens!.items![1]!.valueUsd = null;
    snapshot.scanners.prices = {
      status: 'partial',
      reason: '2/3 mints priced; unquoted mints remain unvalued',
    };
    desktop(() => snapshot);
    const user = userEvent.setup();
    render(<App />);

    // Act
    await connect(user);
    await screen.findByText('1 SOL = $120.50');
    await go(user, 'Swappable tokens');

    // Assert
    expect(screen.getByText('$1.23E-10')).toBeVisible();
    expect(screen.getByText('PRICE UNAVAILABLE')).toBeVisible();
    expect(screen.getByText('2/3 mints priced; unquoted mints remain unvalued')).toBeVisible();
  });
});
