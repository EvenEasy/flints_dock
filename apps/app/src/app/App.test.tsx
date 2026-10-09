import { act, render, screen } from '@testing-library/react';
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
      const source = (payload as { request: { source: { kind: string; address?: string } } })
        .request.source;
      return {
        walletAddress: source.address ?? address,
        sourceKind: source.kind,
        canSign: source.kind !== 'publicKey',
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
async function ready() {
  return screen.findByRole('button', { name: 'ПОВЕРНУТИ SOL' });
}
async function inventory(user: ReturnType<typeof userEvent.setup>) {
  if (screen.queryByRole('dialog'))
    await user.click(screen.getByRole('button', { name: 'Close dialog' }));
  await user.click(screen.getByRole('button', { name: 'ТРЮМИ' }));
}

// These tests exercise actual contract requests; JSX assertions follow the five replacement screens.
describe('Wallet contract and local session', () => {
  it('InvalidAddress_ValidatesBeforeIPC', async () => {
    const ipc = vi.fn(() => wallet());
    desktop(ipc);
    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByRole('button', { name: 'CONNECT WALLET' }));
    await user.click(screen.getByRole('radio', { name: 'Public key (read only)' }));
    await user.type(screen.getByRole('textbox', { name: 'WALLET PUBLIC KEY' }), 'not-a-wallet');
    await user.click(screen.getByRole('button', { name: 'SCAN WALLET' }));
    expect(screen.getByRole('alert')).toHaveTextContent('valid Solana public key');
    expect(ipc).not.toHaveBeenCalled();
  });
  it('PublicAddress_UsesContractAndPreservesExactBalances', async () => {
    const ipc = vi.fn(() => wallet());
    desktop(ipc);
    const user = userEvent.setup();
    render(<App />);
    await connect(user);
    await ready();
    expect(ipc).toHaveBeenCalledExactlyOnceWith('analyze_wallet', {
      request: {
        walletAddress: address,
        noPrices: false,
        selection: { balance: true, tokens: true, allTokens: true, nfts: true, cnfts: true },
      },
    });
    await user.click(screen.getByRole('button', { name: 'ПРОФІЛЬ' }));
    expect(screen.getByText('12.345678901 SOL')).toBeVisible();
    await inventory(user);
    expect(screen.getAllByText('900,719,925,474.099312345')).toHaveLength(2);
    expect(screen.getAllByText('PRICE UNAVAILABLE')).toHaveLength(2);
  });
  it('Browser_DoesNotCollectCredentials', async () => {
    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByRole('button', { name: 'CONNECT WALLET' }));
    expect(screen.getByText('DESKTOP APP REQUIRED')).toBeVisible();
    expect(screen.queryByLabelText('SEED BASE64')).not.toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'EXPLORE DESIGN PREVIEW' }));
    expect(screen.getByText('DESIGN PREVIEW · NO TRANSACTIONS')).toBeVisible();
  });
  it('ContractFailure_ShowsSafeMessageAndReconnection', async () => {
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
    expect(screen.getByRole('heading', { name: 'АНАЛІЗ ПЕРЕРВАНО' })).toBeVisible();
    await user.click(screen.getByRole('button', { name: 'CHANGE WALLET' }));
    await user.click(screen.getByRole('radio', { name: 'Public key (read only)' }));
    expect(screen.getByRole('textbox', { name: 'WALLET PUBLIC KEY' })).toHaveValue(address);
  });
  it('Retry_DoesNotReplayCredentials', async () => {
    const ipc = vi
      .fn()
      .mockRejectedValueOnce('analyze_wallet not allowed')
      .mockResolvedValueOnce(wallet());
    desktop(ipc);
    const user = userEvent.setup();
    render(<App />);
    await connect(user);
    expect(await screen.findByRole('alert')).toHaveTextContent('analyze_wallet not allowed');
    await user.click(screen.getByRole('button', { name: 'RETRY SCAN' }));
    await ready();
    expect(ipc).toHaveBeenCalledTimes(2);
  });
  it('DismissedScan_DiscardsLateResultAndDoesNotFakeProgress', async () => {
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
    expect(screen.queryByText('ЕТАП 3 ІЗ 5')).not.toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'DISMISS SCAN' }));
    await act(async () => resolve(wallet()));
    expect(screen.getByRole('button', { name: 'CONNECT WALLET' })).toBeVisible();
    expect(screen.queryByText('АНАЛІЗ ЗАВЕРШЕНО')).not.toBeInTheDocument();
  });
  it('PartialSnapshot_SurvivesHashChangeAndPreservesFailureState', async () => {
    desktop(() =>
      wallet({
        hasUsableResults: false,
        scanners: { native_sol: { status: 'failed', reason: 'RPC timeout' } },
      }),
    );
    const user = userEvent.setup();
    render(<App />);
    await connect(user);
    await ready();
    await act(async () => window.dispatchEvent(new HashChangeEvent('hashchange')));
    expect(screen.getByRole('alert')).toHaveTextContent('No usable asset results');
    expect(screen.getByRole('button', { name: 'ПОВЕРНУТИ SOL' })).toBeVisible();
  });
  it('Seed_IsTransferredOnceAndNeverStored', async () => {
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
    await ready();
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
    await user.click(screen.getByRole('button', { name: 'ПРОФІЛЬ' }));
    expect(screen.getByText('LOCAL SIGNER CONNECTED')).toBeVisible();
  });
  it('KeypairPath_IsReadOnlyByRust', async () => {
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
    await ready();
    expect(ipc.mock.calls[0]).toEqual([
      'connect_wallet',
      { request: { source: { kind: 'keypairFile', path: '/home/example/id.json' } } },
    ]);
  });
  it('RejectedSeed_ClearsSecretWithoutClosingDialog', async () => {
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
    await user.type(
      screen.getByLabelText('SEED BASE64'),
      'AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=',
    );
    await user.click(screen.getByRole('button', { name: 'SCAN WALLET' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('Seed could not be loaded.');
    expect(screen.getByLabelText('SEED BASE64')).toHaveValue('');
    expect(screen.getByRole('dialog')).toBeVisible();
  });
  it('Disconnect_ForgetsSignerAndResetsView', async () => {
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
    await ready();
    await user.click(screen.getByRole('button', { name: 'ПРОФІЛЬ' }));
    await user.click(screen.getByRole('button', { name: 'DISCONNECT WALLET' }));
    expect(await screen.findByRole('button', { name: 'CONNECT WALLET' })).toBeVisible();
    expect(ipc.mock.calls.map((call) => call[0])).toEqual([
      'connect_wallet',
      'analyze_wallet',
      'disconnect_wallet',
    ]);
  });
  it('OlderBridge_WorksWithoutRuntimeFlag', async () => {
    const ipc = vi.fn(() => wallet());
    mockIPC((command) =>
      command === 'connect_wallet'
        ? { walletAddress: address, sourceKind: 'publicKey', canSign: false }
        : ipc(),
    );
    const user = userEvent.setup();
    render(<App />);
    await connect(user);
    await ready();
    expect(ipc).toHaveBeenCalledTimes(1);
  });
});

describe('Real asset selection and available results', () => {
  it('SameNameDifferentMints_UncheckedMeansKeepOnlyThatMintAndSurvivesNavigation', async () => {
    const ipc = vi.fn(() => wallet());
    desktop(ipc);
    const user = userEvent.setup();
    render(<App />);
    await connect(user);
    await ready();
    await user.click(screen.getByRole('button', { name: 'ПОВЕРНУТИ SOL' }));
    const first = screen.getByRole('checkbox', { name: `Include Same name mint ${address}` });
    const second = screen.getByRole('checkbox', { name: `Include Same name mint ${secondMint}` });
    expect(first).toBeChecked();
    expect(second).toBeChecked();
    await user.click(first);
    expect(first).not.toBeChecked();
    expect(second).toBeChecked();
    await user.click(screen.getByRole('button', { name: 'СТАНЦІЯ' }));
    await user.click(screen.getByRole('button', { name: 'ПОВЕРНУТИ SOL' }));
    expect(
      screen.getByRole('checkbox', { name: `Include Same name mint ${address}` }),
    ).not.toBeChecked();
    expect(ipc).toHaveBeenCalledTimes(1);
  });
  it('Cleanup_DisablesExecutionAndNeverEquatesMissingPriceWithDead', async () => {
    const ipc = vi.fn(() => wallet());
    desktop(ipc);
    const user = userEvent.setup();
    render(<App />);
    await connect(user);
    await ready();
    await user.click(screen.getByRole('button', { name: 'ПОВЕРНУТИ SOL' }));
    expect(screen.getByRole('button', { name: /очищення недоступне/ })).toBeDisabled();
    expect(screen.queryByText('МЕРТВИЙ')).not.toBeInTheDocument();
    expect(screen.queryByText('≈ 0.428 SOL')).not.toBeInTheDocument();
    expect(ipc).toHaveBeenCalledTimes(1);
  });
  it('UntrustedMetadata_RemainsEscapedAndCannotLoadRemoteArtwork', async () => {
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
    await ready();
    await inventory(user);
    expect(screen.getByText('<img src=x onerror=alert(1)>')).toBeVisible();
    expect(screen.getByText('Some metadata unavailable.')).toBeVisible();
    expect(
      [...document.querySelectorAll('img')].every((img) => !img.src.includes('untrusted.invalid')),
    ).toBe(true);
  });
  it.each([
    { items: [], message: 'No token assets found in this category.' },
    { items: null, message: 'Token inventory is unavailable. This is not an empty wallet.' },
  ])('Inventory_$message', async ({ items, message }) => {
    desktop(() => wallet({ tokens: { status: { status: 'complete' }, items } }));
    const user = userEvent.setup();
    render(<App />);
    await connect(user);
    await ready();
    await inventory(user);
    expect(screen.getByText(message)).toBeVisible();
  });
  it('LargeInventory_BoundsRenderedRowsWithoutDroppingSnapshot', async () => {
    const assets = Array.from({ length: 51 }, (_, index) => token(`mint-${index}`));
    desktop(() => wallet({ tokens: { status: { status: 'complete' }, items: assets } }));
    const user = userEvent.setup();
    render(<App />);
    await connect(user);
    await ready();
    await inventory(user);
    expect(screen.getAllByText('PRICE UNAVAILABLE')).toHaveLength(50);
    await user.click(screen.getByRole('button', { name: 'NEXT' }));
    expect(screen.getAllByText('PRICE UNAVAILABLE')).toHaveLength(1);
    await user.click(screen.getByRole('button', { name: 'PREVIOUS' }));
    expect(screen.getAllByText('PRICE UNAVAILABLE')).toHaveLength(50);
  });
  it('CoreAndCompressedNFT_CapabilitiesStayAvailable', async () => {
    desktop(() => wallet());
    const user = userEvent.setup();
    render(<App />);
    await connect(user);
    await ready();
    await inventory(user);
    await user.click(screen.getByRole('button', { name: 'NFT / CORE' }));
    expect(screen.getByText('No classic NFTs found.')).toBeVisible();
    expect(screen.getByText('No Core assets found.')).toBeVisible();
    await user.click(screen.getByRole('button', { name: 'cNFT' }));
    expect(screen.getByText('Historical owner index required.')).toBeVisible();
  });
  it('LiveHash_CannotFabricateCleanupCompletion', async () => {
    desktop(() => wallet());
    const user = userEvent.setup();
    render(<App />);
    await connect(user);
    await ready();
    await act(async () => {
      window.location.hash = 'success';
      window.dispatchEvent(new HashChangeEvent('hashchange'));
    });
    expect(screen.queryByText('ОЧИЩЕННЯ ЗАВЕРШЕНО')).not.toBeInTheDocument();
    expect(screen.getByText('АНАЛІЗ ЗАВЕРШЕНО')).toBeVisible();
  });
});

describe('Live route guards', () => {
  it('ScanningHash_RequiresAnActiveAnalysisRequest', async () => {
    render(<App />);
    await act(async () => {
      window.location.hash = 'scanning';
      window.dispatchEvent(new HashChangeEvent('hashchange'));
    });
    expect(screen.getByRole('button', { name: 'CONNECT WALLET' })).toBeVisible();
    expect(screen.queryByText('ГАМАНЕЦЬ ПІДКЛЮЧЕНО')).not.toBeInTheDocument();
  });
});

describe('Reference preview', () => {
  it.each(screens)('$id_RendersWithoutIPC', ({ id }) => {
    const ipc = vi.fn();
    mockIPC(ipc);
    window.history.replaceState(null, '', `/?preview=1#${id}`);
    render(<App />);
    expect(screen.getByRole('heading', { level: 1 })).toBeVisible();
    expect(screen.getByText('DESIGN PREVIEW · NO TRANSACTIONS')).toBeVisible();
    expect(ipc).not.toHaveBeenCalled();
  });
  it('Selection_UsesXMLDefaultsAndChangesCountWithoutSendingTransactions', async () => {
    const ipc = vi.fn();
    mockIPC(ipc);
    window.history.replaceState(null, '', '/?preview=1#cleanup');
    const user = userEvent.setup();
    render(<App />);
    expect(screen.getByText('4 АКТИВИ')).toBeVisible();
    expect(screen.getByRole('checkbox', { name: /Include USD Coin/ })).not.toBeChecked();
    expect(screen.getByRole('checkbox', { name: /Include Bonk/ })).toBeChecked();
    await user.click(screen.getByRole('checkbox', { name: /Include Bonk/ }));
    expect(screen.getByText('3 АКТИВІВ')).toBeVisible();
    expect(ipc).not.toHaveBeenCalled();
  });
});

describe('Jupiter price presentation', () => {
  it('Prices_PreserveNativeUnitPricesTinyTokenPricesAndHoldingValues', async () => {
    const ipc = vi.fn(() => pricedWallet());
    desktop(ipc);
    const user = userEvent.setup();
    render(<App />);
    await connect(user);
    await ready();
    await user.click(screen.getByRole('button', { name: 'ПРОФІЛЬ' }));
    expect(screen.getByText('1 SOL = $120.50')).toBeVisible();
    expect(screen.getByText('≈ $1,487.65')).toBeVisible();
    await inventory(user);
    expect(screen.getByText('$1.23E-10')).toBeVisible();
    expect(screen.getByText('$1.0022')).toBeVisible();
    expect(screen.getByText('$10.02')).toBeVisible();
    await user.click(screen.getByRole('button', { name: 'ALL ACCOUNTS' }));
    expect(screen.getByText('$1.23E-10')).toBeVisible();
    expect(ipc).toHaveBeenCalledTimes(1);
  });
  it('PricingOptOut_PreservesUnpricedInventory', async () => {
    const ipc = vi.fn(() =>
      wallet({ scanners: { prices: { status: 'skipped', reason: 'Prices disabled by user.' } } }),
    );
    desktop(ipc);
    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByRole('button', { name: 'CONNECT WALLET' }));
    const prices = screen.getByRole('checkbox', { name: 'Include Jupiter USD prices' });
    expect(prices).toBeChecked();
    await user.click(prices);
    await user.click(screen.getByRole('radio', { name: 'Public key (read only)' }));
    await user.type(screen.getByRole('textbox', { name: 'WALLET PUBLIC KEY' }), address);
    await user.click(screen.getByRole('button', { name: 'SCAN WALLET' }));
    await ready();
    expect(ipc).toHaveBeenCalledWith('analyze_wallet', {
      request: expect.objectContaining({ noPrices: true }),
    });
    await inventory(user);
    expect(screen.getAllByText('PRICE UNAVAILABLE')).toHaveLength(2);
  });
  it('PartialPricing_ExposesAvailableQuotesAndDiagnostics', async () => {
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
    await connect(user);
    await ready();
    await inventory(user);
    expect(screen.getByText('$1.23E-10')).toBeVisible();
    expect(screen.getByText('PRICE UNAVAILABLE')).toBeVisible();
    expect(screen.getByText('2/3 mints priced; unquoted mints remain unvalued')).toBeVisible();
  });
});
