import { useNetworkQuality, sendNetworkQualityToSw } from '@nvbes/web-runtime';
import { useEffect } from 'react';

export function NetworkQualityInit() {
  const quality = useNetworkQuality();

  useEffect(() => {
    void sendNetworkQualityToSw(quality);
  }, [quality]);

  return null;
}
