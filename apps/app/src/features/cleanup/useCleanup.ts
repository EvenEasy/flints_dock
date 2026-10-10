import { useCallback, useEffect, useRef, useState } from 'react';
import {
  prepareCleanup,
  executeCleanup,
  getCleanupJob,
  getCleanupPlanJob,
} from '../../../frontend-contract/cleanup';
import type { CleanupJob, CleanupPlan, CleanupProgress } from '../../../frontend-contract/cleanup';
import { readableError } from '../../shared/api/wallet';

/** Selection generations invalidate both the displayed plan and late read-only IPC responses. */
export function useCleanup(
  sessionId: string | undefined,
  enabled: boolean,
  selected: string[],
  ignored: ReadonlySet<string>,
) {
  const [plan, setPlan] = useState<CleanupPlan | null>(null);
  const [job, setJob] = useState<CleanupJob | null>(null);
  const [progress, setProgress] = useState<CleanupProgress | null>(null);
  const [skippedStages, setSkippedStages] = useState<string[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [preparing, setPreparing] = useState(false);
  const [running, setRunning] = useState(false);
  const generation = useRef(0);
  const revision = useRef(0);
  const activeJob = useRef<string | null>(null);
  const activePlan = useRef<string | null>(null);
  const executing = useRef(false);
  const session = useRef(sessionId);
  const [planningProgress, setPlanningProgress] = useState<CleanupProgress | null>(null);
  const [retry, setRetry] = useState(0);
  const selectionKey = JSON.stringify([...selected].sort());
  const ignoredKey = JSON.stringify([...ignored].sort());
  useEffect(() => {
    session.current = sessionId;
    const current = ++generation.current;
    if (!enabled || !sessionId || executing.current) return;
    const selectedMints: string[] = JSON.parse(selectionKey) as string[];
    const ignoredMints: string[] = JSON.parse(ignoredKey) as string[];
    const requestRevision = ++revision.current;
    const defer = window.setTimeout(() => {
      setPlan(null);
      setError(null);
      setPreparing(true);
      setPlanningProgress(null);
      void prepareCleanup(
        {
          sessionId,
          revision: requestRevision,
          selection: selectedMints.length
            ? { mode: 'selected', assetIds: selectedMints }
            : { mode: 'none' },
          ignoredMints: [],
          ignoredAssetIds: ignoredMints,
        },
        (event) => {
          if (generation.current === current && event.sessionId === sessionId)
            setPlanningProgress((previous) =>
              previous && previous.sequence >= event.sequence ? previous : event,
            );
        },
      )
        .then((result) => {
          if (
            generation.current === current &&
            result.revision === requestRevision &&
            result.sessionId === sessionId
          )
            setPlan(result);
        })
        .catch((failure: unknown) => {
          if (generation.current === current) setError(readableError(failure).message);
        })
        .finally(() => {
          if (generation.current === current) setPreparing(false);
        });
    }, 0);
    return () => {
      window.clearTimeout(defer);
      generation.current += 1;
    };
  }, [enabled, sessionId, selectionKey, ignoredKey, retry]);

  const waitForJob = useCallback(
    async (initial: CleanupJob): Promise<CleanupJob | null> => {
      if (!sessionId || initial.sessionId !== sessionId) return null;
      let result = initial;
      activeJob.current = result.jobId;
      while (result.status === 'running' && session.current === sessionId) {
        setJob(result);
        setSkippedStages((previous) => [
          ...new Set([
            ...previous,
            ...result.progress
              .filter((event) => event.status === 'skipped')
              .map((event) => event.stage),
          ]),
        ]);
        const latest = result.progress.at(-1);
        if (latest)
          setProgress((previous) =>
            previous && previous.sequence >= latest.sequence ? previous : latest,
          );
        await new Promise<void>((resolve) => window.setTimeout(resolve, 1000));
        result = await getCleanupJob(sessionId, result.jobId);
      }
      if (session.current !== sessionId || result.sessionId !== sessionId) return null;
      setJob(result);
      return result;
    },
    [sessionId],
  );

  const execute = useCallback(async (): Promise<CleanupJob | null> => {
    if (!plan || !sessionId || executing.current || !plan.canExecute || selected.length === 0)
      return null;
    executing.current = true;
    setRunning(true);
    setError(null);
    setJob(null);
    setProgress(null);
    setSkippedStages([]);
    activeJob.current = null;
    activePlan.current = plan.planId;
    const onProgress = (event: CleanupProgress) => {
      if (
        session.current !== sessionId ||
        event.sessionId !== sessionId ||
        (activeJob.current && activeJob.current !== event.jobId)
      )
        return;
      activeJob.current = event.jobId;
      if (event.status === 'skipped')
        setSkippedStages((previous) =>
          previous.includes(event.stage) ? previous : [...previous, event.stage],
        );
      setProgress((previous) =>
        previous && previous.sequence >= event.sequence ? previous : event,
      );
    };
    try {
      return await waitForJob(await executeCleanup(sessionId, plan.planId, onProgress));
    } catch (failure: unknown) {
      // A lost IPC reply may still represent a submitted job, even without a delivered event.
      try {
        const saved = activeJob.current
          ? await getCleanupJob(sessionId, activeJob.current)
          : await getCleanupPlanJob(sessionId, plan.planId);
        return await waitForJob(saved);
      } catch {
        setError(readableError(failure).message);
        return null;
      }
    } finally {
      executing.current = false;
      setRunning(false);
      setPlan(null);
    }
  }, [plan, sessionId, selected.length, waitForJob]);
  const recover = async (): Promise<CleanupJob | null> => {
    if (!sessionId || !activePlan.current || executing.current) return null;
    executing.current = true;
    setRunning(true);
    setError(null);
    try {
      const saved = activeJob.current
        ? await getCleanupJob(sessionId, activeJob.current)
        : await getCleanupPlanJob(sessionId, activePlan.current);
      return await waitForJob(saved);
    } catch (failure: unknown) {
      if (session.current === sessionId) setError(readableError(failure).message);
      return null;
    } finally {
      executing.current = false;
      setRunning(false);
    }
  };
  return {
    plan,
    job,
    progress,
    error,
    preparing,
    planningProgress,
    skippedStages,
    running,
    execute,
    recover,
    refresh: () => setRetry((value) => value + 1),
    invalidate: () => {
      generation.current += 1;
      setPlan(null);
    },
  };
}
