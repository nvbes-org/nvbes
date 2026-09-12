export interface ClientErrorReportContext {
  tags: {
    feature: string;
    source: 'error_boundary';
  };
}

export interface ClientErrorReporter {
  captureException: (error: Error, context: ClientErrorReportContext) => void;
}

let clientErrorReporter: ClientErrorReporter | undefined;

export function configureErrorReporting(opts: { reporter?: ClientErrorReporter | null }) {
  if ('reporter' in opts) {
    clientErrorReporter = opts.reporter ?? undefined;
  }
}

export function canReportClientError(): boolean {
  return Boolean(clientErrorReporter);
}

export async function reportClientError(feature: string, error: Error): Promise<void> {
  tryClientErrorReporter(error, feature);
}

function tryClientErrorReporter(error: Error, feature: string) {
  if (clientErrorReporter) {
    clientErrorReporter.captureException(error, { tags: { feature, source: 'error_boundary' } });
  }
}
