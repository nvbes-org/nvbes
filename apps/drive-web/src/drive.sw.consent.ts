import { getSwReady } from '@nvbes/web-runtime';
import { isVendorAccepted } from './tracking-consent';

const SENTRY_CONSENT_MESSAGE = 'SENTRY_CONSENT_UPDATED';

export async function syncDriveServiceWorkerSentryConsent() {
  const registration = await getSwReady();
  const activeWorker = registration?.active;
  if (!activeWorker) {
    return;
  }

  activeWorker.postMessage({
    type: SENTRY_CONSENT_MESSAGE,
    sentryAccepted: isVendorAccepted('sentry'),
  });
}
