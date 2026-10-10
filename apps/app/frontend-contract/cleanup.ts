import { invoke } from '@tauri-apps/api/core';
import { progressChannel } from './channel';
/** NONE is explicit; an empty selected array is also NONE, never ALL. */
export type CleanupSelection =
  { mode: 'all' } | { mode: 'selected'; mints: string[] } | { mode: 'none' };
export type CleanupPolicy = 'auto' | 'explicitDiscard';
export type CleanupOperation = 'Swap' | 'Burn' | 'Close';
export interface CleanupProgress {
  jobId: string;
  sessionId: string;
  sequence: number;
  stage: string;
  completed: number;
  total: number;
  operation: CleanupOperation | null;
  account: string | null;
  status: string;
}
export interface CleanupEntry {
  account: string;
  mint: string;
  program: string;
  rawAmount: string;
  action: 'swap' | 'burn' | 'close' | 'skip';
  reason: string;
  reasonCode: string;
  decimals: number | null;
  supply: string | null;
  kind: string;
  mintAuthority: string | null;
  freezeAuthority: string | null;
  closeAuthority: string | null;
  accountExtensions: string[];
  mintExtensions: string[];
  expectedOutLamports: string | null;
  minOutLamports: string | null;
}
export interface CleanupPlan {
  planId: string;
  sessionId: string;
  revision: number;
  network: string;
  expiresAt: string;
  entries: CleanupEntry[];
  policy: CleanupPolicy;
  selectedAssets: number;
  executableAccounts: number;
  skippedAccounts: number;
  undecodableAccounts: {
    address: string;
    program: string | null;
    lamports: string | null;
    reasonCode: 'undecodable';
    reason: string;
  }[];
  canExecute: boolean;
  requiresBurn: boolean;
  swapCount: number;
  burnCount: number;
  closeCount: number;
  estimatedSwapLamports: string;
  estimatedReclaimedLamports: string;
  maxPriceImpactBps: number;
  maxPriorityFeeLamports: string;
  slippageBps: number;
}
export interface OperationReceipt {
  operation: CleanupOperation;
  signature: string;
  wallet_delta_lamports: string | null;
  reclaimed_lamports: string | null;
}
export interface AccountResult {
  token_account: string;
  mint: string;
  category: string;
  status: string;
  reason: string;
  operations: OperationReceipt[];
  uncertain_signature: string | null;
}
/** Core report naming is retained; every on-chain amount is an exact decimal string. */
export interface CleanupReport {
  results: AccountResult[];
  closed: number;
  failed: number;
  skipped: number;
  known_swap_net_lamports: string;
  known_reclaimed_lamports: string;
  known_net_wallet_lamports: string;
  accounting_complete: boolean;
}
export interface CleanupJob {
  jobId: string;
  sessionId: string;
  network: string;
  status: 'running' | 'completed' | 'partial' | 'failed';
  sequence: number;
  progress: CleanupProgress[];
  report: CleanupReport | null;
  error: string | null;
}
export interface PrepareCleanupRequest {
  sessionId: string;
  revision: number;
  selection: CleanupSelection;
  ignoredMints: string[];
  policy?: CleanupPolicy;
}
export function prepareCleanup(
  request: PrepareCleanupRequest,
  onProgress?: (event: CleanupProgress) => void,
): Promise<CleanupPlan> {
  const progress = progressChannel(onProgress);
  return invoke('prepare_cleanup', { request, ...(progress ? { progress } : {}) });
}
/** The backend signs only its own stored plan after explicit action approval. */
export function executeCleanup(
  sessionId: string,
  planId: string,
  onProgress: (event: CleanupProgress) => void,
): Promise<CleanupJob> {
  const channel = progressChannel(onProgress);
  return invoke('execute_cleanup', {
    request: { sessionId, planId, approval: { swap: true, burn: true, close: true } },
    progress: channel,
  });
}
export function getCleanupJob(sessionId: string, jobId: string): Promise<CleanupJob> {
  return invoke('get_cleanup_job', { request: { sessionId, jobId } });
}

/** Resolve a claimed job after IPC loss before its first progress event arrived. Read-only lookup. */
export function getCleanupPlanJob(sessionId: string, planId: string): Promise<CleanupJob> {
  return invoke('get_cleanup_job', { request: { sessionId, planId } });
}
