import type { CleanupPlan, CleanupJob } from '../../frontend-contract/cleanup';
import { address, secondMint } from './fixtures';

/** Nonsecret execution fixtures model Rust reports, including fees and exact values above 2^53. */
export function cleanupPlan(revision = 1, mints = [address, secondMint]): CleanupPlan {
  return {
    planId: `plan-${revision}`,
    sessionId: 'session',
    revision,
    network: 'mock-genesis',
    expiresAt: '9999999999',
    selectedAssets: mints.length,
    executableAccounts: mints.length,
    skippedAccounts: 0,
    undecodableAccounts: [],
    entries: mints.map((mint, i) => ({
      account: `account-${mint}`,
      mint,
      program: 'legacy',
      rawAmount: '1000000',
      action: i === 0 ? 'swap' : 'burn',
      reason: 'fixture planned action',
      reasonCode: i === 0 ? 'swap_route' : 'no_route',
      decimals: 6,
      supply: '1000000',
      kind: 'Fungible',
      mintAuthority: null,
      freezeAuthority: null,
      closeAuthority: null,
      accountExtensions: [],
      mintExtensions: [],
      expectedOutLamports: i === 0 ? '9007199254740993' : null,
      minOutLamports: i === 0 ? '9007199254740000' : null,
    })),
    canExecute: mints.length > 0,
    requiresBurn: mints.length > 1,
    swapCount: mints.length ? 1 : 0,
    burnCount: mints.length > 1 ? 1 : 0,
    closeCount: mints.length,
    estimatedSwapLamports: mints.length ? '9007199254740993' : '0',
    estimatedReclaimedLamports: '4078560',
    maxPriceImpactBps: 500,
    maxPriorityFeeLamports: '100000',
    slippageBps: 50,
  };
}
export function cleanupJob(overrides: Partial<CleanupJob> = {}): CleanupJob {
  return {
    jobId: 'job',
    sessionId: 'session',
    network: 'mock-genesis',
    status: 'completed',
    sequence: 5,
    progress: [],
    error: null,
    report: {
      results: [
        {
          token_account: 'source',
          mint: address,
          category: 'Swappable',
          status: 'closed',
          reason: 'confirmed',
          uncertain_signature: null,
          operations: [
            {
              operation: 'Swap',
              signature: 'fixture-signature',
              wallet_delta_lamports: '9007199254740993',
              reclaimed_lamports: null,
            },
          ],
        },
      ],
      closed: 2,
      failed: 0,
      skipped: 0,
      known_swap_net_lamports: '9007199254740993',
      known_reclaimed_lamports: '4078560',
      known_net_wallet_lamports: '9007199258804553',
      accounting_complete: true,
    },
    ...overrides,
  };
}
