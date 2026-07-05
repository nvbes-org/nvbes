import { SharedTrackingConsentBanner } from '@nvbes/web-runtime';
import { getTrackingConsent, setTrackingConsent } from './tracking-consent';

export function TrackingConsentBanner() {
  return (
    <SharedTrackingConsentBanner
      getTrackingConsent={getTrackingConsent}
      setTrackingConsent={setTrackingConsent}
      sourcePrefix="cloud-web"
      sessionReplayDescription="Relecture masquée, bloquée sur auth, billing et fichiers."
    />
  );
}
