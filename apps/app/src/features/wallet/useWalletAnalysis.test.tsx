import { act, renderHook } from '@testing-library/react';
import { mockIPC } from '@tauri-apps/api/mocks';
import { describe, expect, it } from 'vitest';
import type { WalletAnalysis } from '../../../frontend-contract/types';
import { address, secondMint, wallet } from '../../test/fixtures';
import { useWalletAnalysis } from './useWalletAnalysis';

function pendingScans() {
  const pending: ((result: WalletAnalysis) => void)[] = [];
  Reflect.set(globalThis, 'isTauri', true);
  mockIPC(
    () =>
      new Promise<WalletAnalysis>((resolve) => {
        pending.push(resolve);
      }),
  );
  return pending;
}

describe('Analysis generations', () => {
  it('clears old counters when rescanning and discards the superseded response', async () => {
    const pending = pendingScans();
    const { result } = renderHook(useWalletAnalysis);
    let first!: Promise<boolean>;
    await act(async () => {
      first = result.current.scan({ walletAddress: address });
    });
    await act(async () => {
      pending[0]!(wallet());
      await first;
    });
    expect(result.current.analysis?.owner).toBe(address);
    let old!: Promise<boolean>;
    let latest!: Promise<boolean>;
    await act(async () => {
      old = result.current.scan({ walletAddress: address });
    });
    expect(result.current.analysis).toBeNull();
    await act(async () => {
      latest = result.current.scan({ walletAddress: secondMint });
    });
    await act(async () => {
      pending[2]!(wallet({ owner: secondMint }));
      await latest;
    });
    await act(async () => {
      pending[1]!(wallet());
      await old;
    });
    expect(result.current.analysis?.owner).toBe(secondMint);
    expect(await old).toBe(false);
  });
  it('reset invalidates a late result and clears progress', async () => {
    const pending = pendingScans();
    const { result } = renderHook(useWalletAnalysis);
    let task!: Promise<boolean>;
    await act(async () => {
      task = result.current.scan({ walletAddress: address });
    });
    act(() => result.current.reset());
    await act(async () => {
      pending[0]!(wallet());
      await task;
    });
    expect(result.current.analysis).toBeNull();
    expect(result.current.loading).toBe(false);
    expect(result.current.progress).toBeNull();
  });
  it('rejects an IPC response belonging to another wallet', async () => {
    Reflect.set(globalThis, 'isTauri', true);
    mockIPC(() => wallet({ owner: secondMint }));
    const { result } = renderHook(useWalletAnalysis);
    await act(async () => {
      await result.current.scan({ walletAddress: address });
    });
    expect(result.current.analysis).toBeNull();
    expect(result.current.error?.message).toBe('The analysis returned a different wallet.');
  });
});
