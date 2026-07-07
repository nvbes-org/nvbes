import { useQuery } from '@tanstack/react-query';
import { useEffect, useMemo, useState } from 'react';
import {
  detectRegionQueryFn,
  identityAuthMutationKeys,
  supportedRegionsQueryFn,
} from '../identity.auth.queries';
import { detectRegionFromBrowser } from '../identity.auth.functions';
import type { SupportedRegion } from '../identity.auth.api';

const regionNameCollator = new Intl.Collator('fr', {
  sensitivity: 'base',
  numeric: true,
});

function regionSortLabel(region: SupportedRegion): string {
  return region.display_name ?? region.country_code;
}

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

  const supportedRegions = useMemo(
    () =>
      [...(supportedRegionsQuery.data ?? [])].sort((left, right) =>
        regionNameCollator.compare(regionSortLabel(left), regionSortLabel(right)),
      ),
    [supportedRegionsQuery.data],
  );
  const browserRegionDetection = useMemo(() => {
    if (regionQuery.data?.region || supportedRegions.length === 0) {
      return null;
    }

    return detectRegionFromBrowser(supportedRegions);
  }, [regionQuery.data?.region, supportedRegions]);
  const detectedRegion = regionQuery.data?.region ?? browserRegionDetection?.region ?? null;
  const detectedReliability =
    regionQuery.data?.reliability ?? browserRegionDetection?.reliability ?? 'none';

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
    supportedRegions,
  };
}
