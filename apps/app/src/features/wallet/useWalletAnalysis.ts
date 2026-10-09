import { useCallback, useEffect, useRef, useState } from 'react';
import type { AnalyzeWalletRequest, WalletAnalysis } from '../../../frontend-contract/types';
import { readWallet, readableError } from '../../shared/api/wallet';
import type { ReadError } from '../../shared/api/wallet';

/** Track one read-only analysis and ignore late responses after cancellation or a new request. */
export function useWalletAnalysis() {
  const [analysis, setAnalysis] = useState<WalletAnalysis | null>(null);
  const [error, setError] = useState<ReadError | null>(null);
  const [loading, setLoading] = useState(false);
  const generation = useRef(0);
  const [progress, setProgress] = useState<
    import('../../../frontend-contract/cleanup').CleanupProgress | null
  >(null);

  useEffect(
    () => () => {
      generation.current += 1;
    },
    [],
  );

  const scan = useCallback(async (request: AnalyzeWalletRequest): Promise<boolean> => {
    const current = ++generation.current;
    setAnalysis(null);
    setError(null);
    setLoading(true);
    setProgress(null);
    try {
      const result = await readWallet(request, (event) => {
        if (generation.current === current)
          setProgress((previous) =>
            previous && previous.sequence >= event.sequence ? previous : event,
          );
      });
      if (generation.current !== current) return false;
      setAnalysis(result);
      return true;
    } catch (failure: unknown) {
      if (generation.current === current) setError(readableError(failure));
      return false;
    } finally {
      if (generation.current === current) setLoading(false);
    }
  }, []);

  const reset = useCallback(() => {
    // This dismisses the view, not an RPC operation; the current contract has no cancel command.
    generation.current += 1;
    setAnalysis(null);
    setError(null);
    setLoading(false);
  }, []);

  return { analysis, error, loading, progress, scan, reset };
}
