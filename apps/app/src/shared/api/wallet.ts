import { isTauri } from '@tauri-apps/api/core';
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

/** Use the existing typed IPC boundary. A browser never falls back to direct RPC/HTTP. */
export async function readWallet(request: AnalyzeWalletRequest): Promise<WalletAnalysis> {
  // Older Tauri 2 hosts expose the IPC bridge before the newer isTauri flag.
  const bridge: unknown = Reflect.get(window, '__TAURI_INTERNALS__');
  const hasBridge =
    typeof bridge === 'object' &&
    bridge !== null &&
    typeof Reflect.get(bridge, 'invoke') === 'function';
  if (!isTauri() && !hasBridge) {
    throw {
      code: 'desktop_required',
      message:
        'Live wallet analysis requires the Tauri desktop app. You can explore the interface in Design preview.',
    } satisfies ReadError;
  }
  return analyzeWallet(request);
}

/** Show bounded plain-text errors; do not serialize arbitrary provider/debug objects into UI. */
export function readableError(error: unknown): ReadError {
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
