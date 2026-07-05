import * as Sentry from '@sentry/browser';
import posthog from 'posthog-js';
import {
  getErrorReportingReplaysOnErrorSampleRate,
  getErrorReportingTracesSampleRate,
  scrubErrorReportingBreadcrumb,
  scrubErrorReportingEvent,
} from './error-reporting-privacy';
import type { AnalyticsTransport, FeatureFlagResult, JsonType } from './analytics.types';

export interface BrowserAnalyticsTransportOptions {
  appName: string;
  environment: string;
  sentryDsn?: string;
  sentryTracesSampleRate?: number;
  posthogKey?: string;
  posthogHost?: string;
}

export function createBrowserAnalyticsTransport(
  options: BrowserAnalyticsTransportOptions,
): AnalyticsTransport {
  const sentryEnabled = initSentry(options);
  let posthogInitialized = false;
  let posthogOptedOut = false;

  function ensurePostHog(): boolean {
    if (posthogInitialized) {
      if (posthogOptedOut) {
        posthog.opt_in_capturing();
        posthogOptedOut = false;
      }
      return true;
    }
    posthogInitialized = initPostHog(options);
    return posthogInitialized;
  }

  return {
    trackProductEvent(name, properties) {
      if (!ensurePostHog()) {
        return;
      }
      posthog.capture(name, properties);
    },
    identifyProductUser(userId, traits) {
      if (!ensurePostHog()) {
        return;
      }
      posthog.identify(userId, traits);
    },
    setWorkspaceGroup(workspaceId, traits) {
      if (!ensurePostHog()) {
        return;
      }
      posthog.group('workspace', workspaceId, traits);
    },
    getFeatureFlag(key) {
      if (!ensurePostHog()) {
        return undefined;
      }
      return normalizeFeatureFlagResult(posthog.getFeatureFlag(key));
    },
    getFeatureFlagPayload(key) {
      if (!ensurePostHog()) {
        return undefined;
      }
      return normalizeJson(posthog.getFeatureFlagPayload(key));
    },
    captureException(error, properties) {
      if (!sentryEnabled) {
        return;
      }
      Sentry.captureException(error, { extra: properties });
    },
    startPrivacySafeReplay() {
      if (ensurePostHog()) {
        posthog.startSessionRecording();
      }
    },
    stopPrivacySafeReplay() {
      if (posthogInitialized) {
        posthog.stopSessionRecording();
      }
    },
    disableCapture() {
      if (!posthogInitialized) {
        return;
      }
      posthog.stopSessionRecording();
      posthog.opt_out_capturing();
      posthog.reset(true);
      posthogOptedOut = true;
    },
  };
}

function initSentry(options: BrowserAnalyticsTransportOptions): boolean {
  const dsn = normalizedOptional(options.sentryDsn);
  if (!dsn || typeof window === 'undefined') {
    return false;
  }

  const isProduction = options.environment === 'production';
  Sentry.init({
    dsn,
    environment: options.environment,
    tracesSampleRate:
      options.sentryTracesSampleRate ?? getErrorReportingTracesSampleRate(isProduction),
    replaysOnErrorSampleRate: getErrorReportingReplaysOnErrorSampleRate(isProduction),
    sendDefaultPii: false,
    beforeBreadcrumb: scrubErrorReportingBreadcrumb,
    beforeSend: scrubErrorReportingEvent,
    initialScope: {
      tags: {
        app_name: options.appName,
      },
    },
  });

  return true;
}

function initPostHog(options: BrowserAnalyticsTransportOptions): boolean {
  const token = normalizedOptional(options.posthogKey);
  if (!token || typeof window === 'undefined') {
    return false;
  }

  posthog.init(token, {
    api_host: normalizedOptional(options.posthogHost) ?? 'https://eu.i.posthog.com',
    autocapture: false,
    capture_pageview: false,
    disable_session_recording: true,
    persistence: 'localStorage+cookie',
    person_profiles: 'identified_only',
  });

  return true;
}

function normalizedOptional(value: string | undefined): string | undefined {
  const trimmed = value?.trim();
  return trimmed ? trimmed : undefined;
}

function normalizeFeatureFlagResult(value: unknown): FeatureFlagResult | undefined {
  if (typeof value === 'boolean' || typeof value === 'string' || typeof value === 'number') {
    return value;
  }
  if (value === null) {
    return null;
  }
  return undefined;
}

function normalizeJson(value: unknown): JsonType | undefined {
  if (
    value === null ||
    typeof value === 'string' ||
    typeof value === 'number' ||
    typeof value === 'boolean'
  ) {
    return value;
  }
  if (Array.isArray(value)) {
    const items = value.map(normalizeJson);
    return items.every((item): item is JsonType => item !== undefined) ? items : undefined;
  }
  if (!isRecord(value)) {
    return undefined;
  }

  const normalized: Record<string, JsonType> = {};
  for (const [key, item] of Object.entries(value)) {
    const normalizedItem = normalizeJson(item);
    if (normalizedItem === undefined) {
      return undefined;
    }
    normalized[key] = normalizedItem;
  }
  return normalized;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}
