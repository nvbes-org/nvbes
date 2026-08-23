import { SharedTrackingConsentBanner } from '@nvbes/web-runtime';
import { TrackingConsentToggle } from './components/TrackingConsentToggle';
import { getTrackingConsent, setTrackingConsent } from './tracking-consent';

export function TrackingConsentBanner() {
  return (
    <SharedTrackingConsentBanner
      getTrackingConsent={getTrackingConsent}
      setTrackingConsent={setTrackingConsent}
      sourcePrefix="identity-web"
      ToggleComponent={TrackingConsentToggle}
    />
  );
}
