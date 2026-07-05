import { getSwReady } from '@nvbes/web-runtime';
import { isVendorAccepted } from './tracking-consent';

const ERROR_REPORTING_CONSENT_MESSAGE = 'ERROR_REPORTING_CONSENT_UPDATED';

export async function syncDriveServiceWorkerErrorReportingConsent() {
  const registration = await getSwReady();
  const activeWorker = registration?.active;
  if (!activeWorker) {
    return;
  }

  activeWorker.postMessage({
    type: ERROR_REPORTING_CONSENT_MESSAGE,
    errorReportingAccepted: isVendorAccepted('sentry') || isVendorAccepted('grafana'),
  });
}
