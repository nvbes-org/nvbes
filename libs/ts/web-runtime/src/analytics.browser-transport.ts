import { createSentryReporting } from './analytics.sentry-transport';
import { stripPostHogUrlSecrets } from './analytics.posthog-privacy';
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
  type PostHog = typeof import('posthog-js/dist/module.full.no-external').default;

  const sentry = createSentryReporting(options);
  let posthog: PostHog | null = null;
  let posthogInitializationPromise: Promise<PostHog | null> | null = null;
  let posthogInitialized = false;
  let posthogOptedOut = false;
  let productAnalyticsEnabled = false;

  async function ensurePostHog(): Promise<PostHog | null> {
    if (!productAnalyticsEnabled || typeof window === 'undefined') {
      return null;
    }
    if (posthogInitialized) {
      if (posthogOptedOut && posthog) {
        posthog.opt_in_capturing();
        posthogOptedOut = false;
      }
      return posthog;
    }

    if (!normalizedOptional(options.posthogKey)) {
      reportMissingDevelopmentConfig(options);
      return null;
    }

    posthogInitializationPromise ??= import('posthog-js/dist/module.full.no-external')
      .then((module) => {
        if (!productAnalyticsEnabled) {
          return null;
        }

        const loadedPostHog = module.default;
        posthogInitialized = initPostHog(loadedPostHog, options);
        posthog = posthogInitialized ? loadedPostHog : null;
        return posthog;
      })
      .finally(() => {
        if (!posthogInitialized) {
          posthogInitializationPromise = null;
        }
      });
    return posthogInitializationPromise;
  }

  async function setProductAnalyticsEnabled(enabled: boolean): Promise<void> {
    productAnalyticsEnabled = enabled;
    if (enabled) {
      await ensurePostHog();
      return;
    }

    if (!posthogInitialized || !posthog) {
      return;
    }

    posthog.stopSessionRecording();
    posthog.opt_out_capturing();
    posthog.reset(true);
    posthogOptedOut = true;
  }

  return {
    async trackProductEvent(name, properties) {
      const loadedPostHog = await ensurePostHog();
      if (!loadedPostHog) {
        return;
      }
      loadedPostHog.capture(name, properties);
    },
    async identifyProductUser(userId, traits) {
      const loadedPostHog = await ensurePostHog();
      if (!loadedPostHog) {
        return;
      }
      loadedPostHog.identify(userId, traits);
    },
    async setWorkspaceGroup(workspaceId, traits) {
      const loadedPostHog = await ensurePostHog();
      if (!loadedPostHog) {
        return;
      }
      loadedPostHog.group('workspace', workspaceId, traits);
    },
    async getFeatureFlag(key) {
      const loadedPostHog = await ensurePostHog();
      if (!loadedPostHog) {
        return undefined;
      }
      return normalizeFeatureFlagResult(loadedPostHog.getFeatureFlag(key));
    },
    async getFeatureFlagPayload(key) {
      const loadedPostHog = await ensurePostHog();
      if (!loadedPostHog) {
        return undefined;
      }
      return normalizeJson(loadedPostHog.getFeatureFlagPayload(key));
    },
    async getCorrelationContext() {
      const loadedPostHog = await ensurePostHog();
      if (!loadedPostHog) {
        return undefined;
      }

      const distinctId = loadedPostHog.get_distinct_id();
      const sessionId = loadedPostHog.get_session_id();
      return distinctId && sessionId ? { distinctId, sessionId } : undefined;
    },
    async captureException(error, properties) {
      sentry.captureException(error, properties);
      const loadedPostHog = await ensurePostHog();
      loadedPostHog?.captureException(error, properties);
    },
    async startPrivacySafeReplay() {
      const loadedPostHog = await ensurePostHog();
      if (loadedPostHog) {
        loadedPostHog.startSessionRecording();
      }
    },
    stopPrivacySafeReplay() {
      if (posthogInitialized && posthog) {
        posthog.stopSessionRecording();
      }
    },
    setProductAnalyticsEnabled,
    setErrorReportingEnabled: sentry.setEnabled,
    async disableCapture() {
      await Promise.all([sentry.setEnabled(false), setProductAnalyticsEnabled(false)]);
    },
  };
}

export function createBrowserAnalyticsTransportFromEnv(options: {
  appName: string;
  environment: string;
  env: Record<string, string | boolean | undefined>;
}): AnalyticsTransport {
  return createBrowserAnalyticsTransport({
    appName: options.appName,
    environment: options.environment,
    sentryDsn: stringEnv(options.env, 'VITE_SENTRY_DSN'),
    sentryTracesSampleRate: numberEnv(options.env, 'VITE_SENTRY_TRACES_SAMPLE_RATE'),
    posthogKey: stringEnv(options.env, 'VITE_POSTHOG_KEY'),
    posthogHost: stringEnv(options.env, 'VITE_POSTHOG_HOST'),
  });
}

function reportMissingDevelopmentConfig(options: BrowserAnalyticsTransportOptions): void {
  if (options.environment !== 'development') {
    return;
  }

  console.error(
    `PostHog is enabled for ${options.appName}, but VITE_POSTHOG_KEY is missing or unconfigured. Product analytics events will be silently missed.`,
  );
}

function initPostHog(
  posthog: typeof import('posthog-js/dist/module.full.no-external').default,
  options: BrowserAnalyticsTransportOptions,
): boolean {
  const token = normalizedOptional(options.posthogKey);
  if (!token || typeof window === 'undefined') {
    return false;
  }

  posthog.init(token, {
    api_host: normalizedOptional(options.posthogHost) ?? 'https://eu.i.posthog.com',
    autocapture: false,
    capture_pageleave: true,
    capture_pageview: false,
    before_send: stripPostHogUrlSecrets,
    custom_personal_data_properties: ['token'],
    disable_capture_url_hashes: true,
    disable_session_recording: true,
    mask_personal_data_properties: true,
    persistence: 'localStorage+cookie',
    person_profiles: 'identified_only',
  });

  return true;
}

function normalizedOptional(value: string | undefined): string | undefined {
  const trimmed = value?.trim();
  return trimmed ? trimmed : undefined;
}

function stringEnv(
  env: Record<string, string | boolean | undefined>,
  key: string,
): string | undefined {
  const value = env[key];
  return typeof value === 'string' ? value : undefined;
}

function numberEnv(
  env: Record<string, string | boolean | undefined>,
  key: string,
): number | undefined {
  const value = stringEnv(env, key);
  if (!value) {
    return undefined;
  }
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : undefined;
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
