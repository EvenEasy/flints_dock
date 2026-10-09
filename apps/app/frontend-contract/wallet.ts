import { invoke } from '@tauri-apps/api/core';
import { progressChannel } from './channel';
import type { AnalyzeWalletRequest, WalletAnalysis } from './types';

/** Central command names for the public IPC boundary. */
export const COMMANDS = {
  analyzeWallet: 'analyze_wallet',
} as const;

/** Read selected holdings; command rejections use AppError, partial reads use scanner statuses. */
export async function analyzeWallet(
  request: AnalyzeWalletRequest,
  onProgress?: (event: import('./cleanup').CleanupProgress) => void,
): Promise<WalletAnalysis> {
  const progress = progressChannel(onProgress);
  return invoke<WalletAnalysis>(COMMANDS.analyzeWallet, {
    request,
    ...(progress ? { progress } : {}),
  });
}
