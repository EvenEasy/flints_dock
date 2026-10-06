import { act, render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { mockIPC } from '@tauri-apps/api/mocks';
import { App } from './App';
import { screens } from './navigation';
import { address, secondMint, token, wallet } from '../test/fixtures';
import type { WalletAnalysis } from '../../frontend-contract/types';

function desktop(handler: Parameters<typeof mockIPC>[0]) {
  Reflect.set(globalThis, 'isTauri', true);
  mockIPC(handler);
}

async function connect(user: ReturnType<typeof userEvent.setup>) {
  await user.click(screen.getByRole('button', { name: 'CONNECT WALLET' }));
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
        noPrices: true,
        selection: { balance: true, tokens: true, allTokens: true, nfts: true, cnfts: true },
      },
    });
    expect(screen.getByText('12.345678901 SOL')).toBeVisible();
    await go(user, 'Swappable tokens');
    expect(screen.getAllByText('900,719,925,474.099312345')).toHaveLength(2);
    expect(screen.getAllByText('NOT QUOTED')).toHaveLength(2);
  });

  it('Scan_BrowserRuntime_ShowsDesktopRequirement', async () => {
    const user = userEvent.setup();
    render(<App />);

    await connect(user);

    expect(await screen.findByRole('alert')).toHaveTextContent('requires the Tauri desktop app');
    expect(screen.getByRole('button', { name: 'CONNECT WALLET' })).toBeEnabled();
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
    await user.click(screen.getByRole('button', { name: 'CONNECT WALLET' }));
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
    mockIPC(ipc);
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
