import { act, renderHook, waitFor } from '@testing-library/react';
import { mockIPC } from '@tauri-apps/api/mocks';
import { expect, it, vi } from 'vitest';
import { useCleanup } from './useCleanup';
import { cleanupPlan, cleanupJob } from '../../test/cleanup';
import { address, secondMint } from '../../test/fixtures';
import type {
  PrepareCleanupRequest,
  CleanupPlan,
  CleanupProgress,
} from '../../../frontend-contract/cleanup';

it('NoneIsExplicitAndIgnoredMintsSurviveTheContract', async () => {
  const ipc = vi.fn((command: string, payload: unknown) => {
    expect(command).toBe('prepare_cleanup');
    const request = (payload as { request: PrepareCleanupRequest }).request;
    expect(request.selection).toEqual({ mode: 'none' });
    expect(request.ignoredAssetIds).toEqual([address, secondMint].sort());
    expect(request.ignoredMints).toEqual([]);
    return cleanupPlan(request.revision, []);
  });
  mockIPC(ipc);
  const { result } = renderHook(() =>
    useCleanup('session', true, [], new Set([address, secondMint])),
  );
  await waitFor(() => expect(result.current.plan?.canExecute).toBe(false));
  await act(async () => {
    expect(await result.current.execute()).toBeNull();
  });
  expect(ipc).toHaveBeenCalledTimes(1);
});
it('LatePlanCannotReplaceNewSelection', async () => {
  const responses: ((plan: CleanupPlan) => void)[] = [];
  mockIPC(
    () =>
      new Promise<CleanupPlan>((resolve) => {
        responses.push(resolve);
      }),
  );
  const { result, rerender } = renderHook(
    ({ selected }) => useCleanup('session', true, selected, new Set()),
    { initialProps: { selected: [address] } },
  );
  await waitFor(() => expect(responses).toHaveLength(1));
  rerender({ selected: [secondMint] });
  await waitFor(() => expect(responses).toHaveLength(2));
  await act(async () => {
    responses[1]!(cleanupPlan(2, [secondMint]));
  });
  await act(async () => {
    responses[0]!(cleanupPlan(1, [address]));
  });
  expect(result.current.plan?.planId).toBe('plan-2');
});
it('DuplicateExecuteAndForeignProgressCannotCreateAnotherJob', async () => {
  let finish: (job: ReturnType<typeof cleanupJob>) => void = () => undefined;
  let emit: (event: CleanupProgress) => void = () => undefined;
  const ipc = vi.fn((command: string, payload: unknown) => {
    if (command === 'prepare_cleanup') return cleanupPlan();
    expect(command).toBe('execute_cleanup');
    emit = (payload as { progress: { onmessage: (event: CleanupProgress) => void } }).progress
      .onmessage;
    return new Promise<ReturnType<typeof cleanupJob>>((resolve) => {
      finish = resolve;
    });
  });
  mockIPC(ipc);
  const { result } = renderHook(() =>
    useCleanup('session', true, [address, secondMint], new Set()),
  );
  await waitFor(() => expect(result.current.plan).not.toBeNull());
  let execution: Promise<ReturnType<typeof cleanupJob> | null>;
  act(() => {
    execution = result.current.execute();
  });
  await act(async () => {
    expect(await result.current.execute()).toBeNull();
  });
  act(() => {
    emit({
      jobId: 'job',
      sessionId: 'session',
      sequence: 3,
      stage: 'close',
      completed: 1,
      total: 2,
      operation: 'Close',
      account: 'a',
      status: 'running',
    });
    emit({
      jobId: 'foreign',
      sessionId: 'session',
      sequence: 99,
      stage: 'completed',
      completed: 2,
      total: 2,
      operation: null,
      account: null,
      status: 'complete',
    });
    emit({
      jobId: 'job',
      sessionId: 'session',
      sequence: 2,
      stage: 'swap',
      completed: 0,
      total: 2,
      operation: 'Swap',
      account: 'a',
      status: 'running',
    });
  });
  expect(result.current.progress?.sequence).toBe(3);
  await act(async () => {
    finish(cleanupJob());
    await execution;
  });
  expect(ipc.mock.calls.filter(([command]) => command === 'execute_cleanup')).toHaveLength(1);
  expect(result.current.job?.report?.known_net_wallet_lamports).toBe('9007199258804553');
});

it('LostInitialReplyQueriesTheSavedPlanWithoutResend', async () => {
  const ipc = vi.fn((command: string, payload: unknown) => {
    if (command === 'prepare_cleanup') return cleanupPlan();
    if (command === 'execute_cleanup') throw new Error('IPC reply lost');
    expect(command).toBe('get_cleanup_job');
    expect(payload).toEqual({ request: { sessionId: 'session', planId: 'plan-1' } });
    return cleanupJob();
  });
  mockIPC(ipc);
  const { result } = renderHook(() =>
    useCleanup('session', true, [address, secondMint], new Set()),
  );
  await waitFor(() => expect(result.current.plan).not.toBeNull());
  await act(async () => {
    await result.current.execute();
  });
  expect(result.current.job?.status).toBe('completed');
  expect(ipc.mock.calls.filter(([command]) => command === 'execute_cleanup')).toHaveLength(1);
});

it('ManualRecoveryKeepsPollingAnExistingJobWithoutResubmission', async () => {
  let lookups = 0;
  const skipped = {
    jobId: 'job',
    sessionId: 'session',
    sequence: 1,
    stage: 'swap',
    completed: 0,
    total: 0,
    operation: null,
    account: null,
    status: 'skipped',
  } satisfies CleanupProgress;
  const ipc = vi.fn((command: string) => {
    if (command === 'prepare_cleanup') return cleanupPlan();
    if (command === 'execute_cleanup') throw new Error('IPC reply lost');
    expect(command).toBe('get_cleanup_job');
    lookups += 1;
    if (lookups === 1) throw new Error('IPC temporarily unavailable');
    if (lookups === 2)
      return { ...cleanupJob(), status: 'running', report: null, progress: [skipped] };
    return cleanupJob();
  });
  mockIPC(ipc);
  const { result } = renderHook(() =>
    useCleanup('session', true, [address, secondMint], new Set()),
  );
  await waitFor(() => expect(result.current.plan).not.toBeNull());
  await act(async () => {
    await result.current.execute();
  });
  expect(result.current.error).toBe('IPC reply lost');
  let recovery!: Promise<ReturnType<typeof cleanupJob> | null>;
  act(() => {
    recovery = result.current.recover();
  });
  await waitFor(() => expect(result.current.skippedStages).toEqual(['swap']));
  await act(async () => {
    expect(await result.current.recover()).toBeNull();
  });
  await act(async () => {
    await recovery;
  });
  expect(result.current.running).toBe(false);
  expect(result.current.job?.status).toBe('completed');
  expect(ipc.mock.calls.filter(([command]) => command === 'execute_cleanup')).toHaveLength(1);
});

it('SelectionChangesInvalidateOlderPlansWithoutAClientPolicy', async () => {
  const responses: ((plan: CleanupPlan) => void)[] = [];
  const requests: PrepareCleanupRequest[] = [];
  mockIPC((command: string, payload: unknown) => {
    expect(command).toBe('prepare_cleanup');
    requests.push((payload as { request: PrepareCleanupRequest }).request);
    return new Promise<CleanupPlan>((resolve) => responses.push(resolve));
  });
  const { result, rerender } = renderHook(
    ({ assets }: { assets: string[] }) => useCleanup('session', true, assets, new Set()),
    { initialProps: { assets: [address] } },
  );
  await waitFor(() => expect(responses).toHaveLength(1));
  rerender({ assets: [] });
  await waitFor(() => expect(responses).toHaveLength(2));
  await act(async () => responses[1]!(cleanupPlan(2, [])));
  await act(async () => responses[0]!(cleanupPlan(1, [address])));
  expect(result.current.plan?.revision).toBe(2);
  expect(requests[1]?.selection).toEqual({ mode: 'none' });
  expect(requests.every((request) => !('policy' in request))).toBe(true);
});
