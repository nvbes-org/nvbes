import { normalizeServiceWorkerError, scrubUnknownValue } from './drive.sw.error.reporting.scrub';

let consentGranted = false;

export function setDriveServiceWorkerErrorReportingConsent(accepted: boolean): boolean {
  consentGranted = accepted;
  return false;
}

export function captureDriveServiceWorkerException(
  error: unknown,
  operation: string,
  context?: Record<string, unknown>,
): void {
  if (!consentGranted) {
    return;
  }

  void normalizeServiceWorkerError(error);
  void scrubUnknownValue({ operation, context });
}
