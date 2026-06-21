import type { ClientErrorReportContext } from '@nvbes/web-runtime';

export function initErrorReporting(): boolean {
  return false;
}

export async function syncErrorReportingConsent(): Promise<void> {}

export function captureErrorReportingException(
  error: Error,
  context: ClientErrorReportContext,
): void {
  void error;
  void context;
}
