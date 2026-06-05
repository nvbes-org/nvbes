interface ErrorReportPayload {
  feature: string;
  message: string;
  stack?: string;
  url: string;
  timestamp: string;
  userAgent: string;
  requestId?: string;
}

export interface ClientErrorReportContext {
  tags: {
    feature: string;
    source: 'error_boundary';
  };
}

export interface ClientErrorReporter {
  captureException: (error: Error, context: ClientErrorReportContext) => void;
}

const CLOUDFLARE_BEACON_URL = '/cdn-cgi/rum';

let cloudflareBeaconEnabled = false;
let clientErrorReporter: ClientErrorReporter | undefined;

export function configureErrorReporting(opts: {
  cloudflare?: boolean;
  reporter?: ClientErrorReporter | null;
}) {
  cloudflareBeaconEnabled = opts.cloudflare ?? false;
  if ('reporter' in opts) {
    clientErrorReporter = opts.reporter ?? undefined;
  }
}

export function canReportClientError(): boolean {
  return cloudflareBeaconEnabled || Boolean(clientErrorReporter);
}

export async function reportClientError(feature: string, error: Error): Promise<void> {
  const payload: ErrorReportPayload = {
    feature,
    message: error.message,
    stack: error.stack,
    url: window.location.href,
    timestamp: new Date().toISOString(),
    userAgent: navigator.userAgent,
  };

  try {
    const requestId = sessionStorage.getItem('nvbes.request-id');
    if (requestId) payload.requestId = requestId;
  } catch {
    // sessionStorage unavailable (private browsing, etc.)
  }

  if (cloudflareBeaconEnabled) {
    sendCloudflareBeacon(payload);
  }

  trySentryReport(error, feature);
}

function sendCloudflareBeacon(payload: ErrorReportPayload) {
  try {
    navigator.sendBeacon(
      CLOUDFLARE_BEACON_URL,
      new Blob([JSON.stringify({ type: 'error', ...payload })], { type: 'application/json' }),
    );
  } catch {
    // Beacon not available
  }
}

function trySentryReport(error: Error, feature: string) {
  if (clientErrorReporter) {
    clientErrorReporter.captureException(error, { tags: { feature, source: 'error_boundary' } });
  }
}
