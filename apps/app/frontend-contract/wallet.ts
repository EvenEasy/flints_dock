import { invoke } from '@tauri-apps/api/core';
import type { AnalyzeWalletRequest, WalletAnalysis } from './types';

/** Central command names for the public IPC boundary. */
export const COMMANDS = {
  analyzeWallet: 'analyze_wallet',
} as const;

/** Read selected holdings; command rejections use AppError, partial reads use scanner statuses. */
export async function analyzeWallet(request: AnalyzeWalletRequest): Promise<WalletAnalysis> {
  return invoke<WalletAnalysis>(COMMANDS.analyzeWallet, { request });
}

