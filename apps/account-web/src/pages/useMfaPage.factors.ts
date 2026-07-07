import type { MfaFactorView } from '@nvbes/identity-sdk-core/src/types';
import { listMfaFactors } from '@nvbes/identity-sdk-web';
import { useCallback, useEffect, useState } from 'react';

export function useMfaFactors() {
  const [factors, setFactors] = useState<MfaFactorView[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchFactors = useCallback(async () => {
    setLoading(true);
    setError(null);

    try {
      const result = await listMfaFactors('');
      setFactors(result.factors);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load MFA factors');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void fetchFactors();
  }, [fetchFactors]);

  return {
    factors,
    loading,
    error,
    setFactors,
  };
}
