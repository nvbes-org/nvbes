import type { MfaFactorView } from '@nvbes/identity-sdk-core/src/types';
import { listMfaFactors } from '@nvbes/identity-sdk-web';
import { useCallback, useEffect, useState } from 'react';

export function useMfaFactors() {
  const [factors, setFactors] = useState<MfaFactorView[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [nextCursor, setNextCursor] = useState<string | null>(null);
  const [hasMore, setHasMore] = useState(false);
  const [loadingMore, setLoadingMore] = useState(false);

  const fetchFactors = useCallback(async () => {
    setLoading(true);
    setError(null);

    try {
      const result = await listMfaFactors('');
      setFactors(result.factors);
      setNextCursor(result.next_cursor);
      setHasMore(result.has_more);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load MFA factors');
    } finally {
      setLoading(false);
    }
  }, []);

  const loadMore = useCallback(async () => {
    if (!nextCursor || loading || loadingMore) return;
    setLoadingMore(true);
    setError(null);
    try {
      const result = await listMfaFactors('', undefined, { cursor: nextCursor });
      setFactors((current) => [...current, ...result.factors]);
      setNextCursor(result.next_cursor);
      setHasMore(result.has_more);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load more MFA factors');
    } finally {
      setLoadingMore(false);
    }
  }, [loading, loadingMore, nextCursor]);

  useEffect(() => {
    void fetchFactors();
  }, [fetchFactors]);

  return {
    factors,
    loading,
    error,
    setFactors,
    hasMore,
    loadMore,
    loadingMore,
  };
}
