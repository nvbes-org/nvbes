import { useQuery } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import {
  detectRegionQueryFn,
  identityAuthMutationKeys,
  supportedRegionsQueryFn,
} from '../identity.auth.queries';

export function useRegisterPageRegions() {
  const [selectedRegion, setSelectedRegion] = useState('');

  const regionQuery = useQuery({
    queryKey: identityAuthMutationKeys.regionDetect,
    queryFn: detectRegionQueryFn,
    staleTime: 0,
    gcTime: 30 * 60 * 1000,
    refetchOnWindowFocus: true,
  });

  const supportedRegionsQuery = useQuery({
    queryKey: identityAuthMutationKeys.supportedRegions,
    queryFn: supportedRegionsQueryFn,
    staleTime: 30 * 60 * 1000,
    gcTime: 30 * 60 * 1000,
    refetchOnWindowFocus: false,
  });

  const detectedRegion = regionQuery.data?.region ?? null;
  const detectedReliability = regionQuery.data?.reliability ?? 'none';

  useEffect(() => {
    if (detectedRegion && !selectedRegion) {
      setSelectedRegion(detectedRegion);
    }
  }, [detectedRegion, selectedRegion]);

  return {
    detectedRegion,
    detectedReliability,
    regionLoading: regionQuery.isPending,
    selectedRegion,
    setSelectedRegion,
    supportedRegions: supportedRegionsQuery.data ?? [],
  };
}
