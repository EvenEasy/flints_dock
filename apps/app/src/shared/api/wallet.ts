import { isTauri } from '@tauri-apps/api/core';
import { connectWallet, disconnectWallet } from '../../../frontend-contract/identity';
import type { ConnectWalletRequest, WalletConnection } from '../../../frontend-contract/types';
import { analyzeWallet } from '../../../frontend-contract/wallet';
import type {
  AnalyzeWalletRequest,
  AppError,
  WalletAnalysis,
} from '../../../frontend-contract/types';

export interface ReadError {
  code: string;
  message: string;
}

/** Detect the desktop IPC transport for both older and current Tauri 2 hosts. */
export function hasDesktopRuntime(): boolean {
  // Older Tauri 2 hosts expose the IPC bridge before the newer isTauri flag.
  const bridge: unknown = Reflect.get(window, '__TAURI_INTERNALS__');
  const hasBridge =
    typeof bridge === 'object' &&
    bridge !== null &&
    typeof Reflect.get(bridge, 'invoke') === 'function';
  return isTauri() || hasBridge;
}

/** Use the existing typed IPC boundary. A browser never falls back to direct RPC/HTTP. */
export async function readWallet(
  request: AnalyzeWalletRequest,
  onProgress?: (event: import('../../../frontend-contract/cleanup').CleanupProgress) => void,
): Promise<WalletAnalysis> {
  if (!hasDesktopRuntime()) {
    throw {
      code: 'desktop_required',
      message:
        'Live wallet analysis requires the Tauri desktop app. Stop npm run dev and run npm run desktop from apps/app, then use the desktop window. Browser mode supports Design preview only.',
    } satisfies ReadError;
  }
  return analyzeWallet(request, onProgress);
}

/** Show bounded plain-text errors; do not serialize arbitrary provider/debug objects into UI. */
export function readableError(error: unknown): ReadError {
  if (typeof error === 'string') {
    // IPC permission failures can be strings rather than the Rust error envelope.
    return {
      code: 'desktop_transport_error',
      message: error.replace(/[\u0000-\u001f\u007f]/g, '').slice(0, 512),
    };
  }
  if (
    typeof error === 'object' &&
    error !== null &&
    'message' in error &&
    typeof error.message === 'string'
  ) {
    const candidate = error as Partial<AppError>;
    return {
      code: typeof candidate.code === 'string' ? candidate.code : 'analysis_failed',
      message: error.message.replace(/[\u0000-\u001f\u007f]/g, '').slice(0, 512),
    };
  }
  return {
    code: 'analysis_failed',
    message: 'Wallet analysis could not be completed. Check the desktop backend and try again.',
  };
}

/** Keep local identity commands behind the same desktop-only boundary as analysis. */
export async function connectLocalWallet(request: ConnectWalletRequest): Promise<WalletConnection> {
  if (!hasDesktopRuntime())
    throw {
      code: 'desktop_required',
      message: 'Wallet credentials are accepted only in the desktop app.',
    } satisfies ReadError;
  return connectWallet(request);
}

/** Release the backend signer without serializing it to the frontend. */
export async function forgetLocalWallet(): Promise<void> {
  if (hasDesktopRuntime()) await disconnectWallet();
}
