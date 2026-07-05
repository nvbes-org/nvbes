import {
  getWebInstrumentations,
  initializeFaro,
  type BeforeSendHook,
  type Faro,
} from '@grafana/faro-web-sdk';
import { TracingInstrumentation } from '@grafana/faro-web-tracing';
import type { ClientErrorReportContext } from '@nvbes/web-runtime';
import { isCategoryAccepted, isVendorAccepted } from './tracking-consent';

const APP_NAME = 'identity-web';

let faroInstance: Faro | null = null;
let enabled = false;

export function initFaro(): boolean {
  const configured = isConfigured();
  if (configured && hasFaroConsent()) {
    enableFaro();
  }
  return configured;
}

export function syncFaroConsent(): void {
  if (!isConfigured()) {
    return;
  }

  if (hasFaroConsent()) {
    enableFaro();
    faroInstance?.unpause();
    enabled = true;
    return;
  }

  enabled = false;
  faroInstance?.pause();
}

export function captureFaroException(error: Error, context: ClientErrorReportContext): void {
  if (!enabled || !faroInstance) {
    return;
  }

  faroInstance.api.pushError(error, {
    type: context.tags.feature,
    context: {
      source: context.tags.source,
      feature: context.tags.feature,
      route_path: currentPath(),
    },
  });
}

function enableFaro(): void {
  if (faroInstance) {
    enabled = true;
    return;
  }

  const url = faroUrl();
  if (!url || typeof window === 'undefined') {
    return;
  }

  enabled = true;
  faroInstance = initializeFaro({
    url,
    apiKey: normalizedOptional(import.meta.env.VITE_FARO_API_KEY),
    app: {
      name: APP_NAME,
      version: releaseName(),
      release: releaseName(),
      environment: import.meta.env.MODE,
    },
    beforeSend: consentGuard,
    trackGeolocation: false,
    sessionTracking: {
      enabled: true,
      persistent: false,
      samplingRate: sampleRateFromEnv(import.meta.env.VITE_FARO_SESSION_SAMPLE_RATE) ?? 1,
    },
    pageTracking: {
      generatePageId: () => currentPath(),
    },
    instrumentations: [
      ...getWebInstrumentations({
        captureConsole: false,
        enableContentSecurityPolicyInstrumentation: true,
      }),
      new TracingInstrumentation({
        resourceAttributes: {
          'service.name': APP_NAME,
          'deployment.environment': import.meta.env.MODE,
        },
        instrumentationOptions: {
          propagateTraceHeaderCorsUrls: tracingOrigins(),
        },
      }),
    ],
  });
}

const consentGuard: BeforeSendHook = (item) => {
  if (!enabled || !hasFaroConsent()) {
    return null;
  }
  return item;
};

function hasFaroConsent(): boolean {
  return isVendorAccepted('grafana') || isCategoryAccepted('performance');
}

function isConfigured(): boolean {
  return Boolean(faroUrl()) && typeof window !== 'undefined';
}

function faroUrl(): string | undefined {
  return (
    normalizedOptional(import.meta.env.VITE_FARO_URL) ??
    normalizedOptional(import.meta.env.VITE_GRAFANA_FARO_URL)
  );
}

function releaseName(): string | undefined {
  return (
    normalizedOptional(import.meta.env.VITE_FARO_RELEASE) ??
    normalizedOptional(import.meta.env.VITE_NVBES_BUILD_ID)
  );
}

function tracingOrigins(): string[] {
  const configured = normalizedOptional(import.meta.env.VITE_FARO_TRACING_ORIGINS);
  if (!configured) {
    return [window.location.origin];
  }

  return configured
    .split(',')
    .map((origin) => origin.trim())
    .filter(Boolean);
}

function currentPath(): string {
  if (typeof window === 'undefined') {
    return '/';
  }
  return window.location.pathname || '/';
}

function normalizedOptional(value: string | undefined): string | undefined {
  const trimmed = value?.trim();
  return trimmed ? trimmed : undefined;
}

function sampleRateFromEnv(value: string | undefined): number | undefined {
  const parsed = Number(value);
  if (!Number.isFinite(parsed)) {
    return undefined;
  }
  return Math.min(Math.max(parsed, 0), 1);
}
