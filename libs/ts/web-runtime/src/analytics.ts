import { configureHttpRequestContextHeaders } from '@nvbes/http-client';
import { hasAnyAnalyticsConsent } from './analytics.privacy';
import { RuntimeAnalytics } from './analytics.runtime';
import type {
  AnalyticsRuntime,
  AnalyticsRuntimeOptions,
  FeatureFlagResult,
  JsonType,
} from './analytics.types';

let runtimeSingleton: RuntimeAnalytics | null = null;

export function initAnalyticsRuntime(options: AnalyticsRuntimeOptions): AnalyticsRuntime {
  runtimeSingleton = new RuntimeAnalytics(options);
  configureHttpRequestContextHeaders(getAnalyticsRequestHeaders);
  void runtimeSingleton.init();
  return runtimeSingleton;
}

export function getAnalyticsRuntime(): AnalyticsRuntime | null {
  return runtimeSingleton;
}

export async function trackProductEvent(
  name: string,
  properties?: Record<string, unknown>,
): Promise<void> {
  await runtimeSingleton?.trackProductEvent(name, properties);
}

export async function identifyProductUser(
  userId: string,
  traits?: Record<string, unknown>,
): Promise<void> {
  await runtimeSingleton?.identifyProductUser(userId, traits);
}

export async function setAnalyticsWorkspaceGroup(
  workspaceId: string,
  traits?: Record<string, unknown>,
): Promise<void> {
  await runtimeSingleton?.setAnalyticsWorkspaceGroup(workspaceId, traits);
}

export async function getFeatureFlag(key: string): Promise<FeatureFlagResult | undefined> {
  return runtimeSingleton?.getFeatureFlag(key);
}

export async function getFeatureFlagPayload(key: string): Promise<JsonType | undefined> {
  return runtimeSingleton?.getFeatureFlagPayload(key);
}

export async function isFeatureEnabled(key: string): Promise<boolean> {
  return runtimeSingleton?.isFeatureEnabled(key) ?? false;
}

export async function trackExperimentExposure(key: string, variant: string): Promise<void> {
  await runtimeSingleton?.trackExperimentExposure(key, variant);
}

export async function getAnalyticsRequestHeaders(): Promise<Record<string, string>> {
  const context = await runtimeSingleton?.getCorrelationContext();
  if (!context) {
    return {};
  }

  return {
    'X-PostHog-Distinct-Id': context.distinctId,
    'X-PostHog-Session-Id': context.sessionId,
  };
}

export async function captureAnalyticsException(
  error: unknown,
  properties?: Record<string, unknown>,
): Promise<void> {
  await runtimeSingleton?.captureAnalyticsException(error, properties);
}

export async function startPrivacySafeReplay(): Promise<void> {
  await runtimeSingleton?.startPrivacySafeReplay();
}

export async function stopPrivacySafeReplay(): Promise<void> {
  await runtimeSingleton?.stopPrivacySafeReplay();
}

export { hasAnyAnalyticsConsent };
export {
  createBrowserAnalyticsTransport,
  createBrowserAnalyticsTransportFromEnv,
} from './analytics.browser-transport';
export type { BrowserAnalyticsTransportOptions } from './analytics.browser-transport';
export {
  ALL_ANALYTICS_CONSENT,
  EMPTY_ANALYTICS_CONSENT,
  type AnalyticsCorrelationContext,
  type AnalyticsPurposeConsent,
  type AnalyticsRuntime,
  type AnalyticsRuntimeOptions,
  type AnalyticsTransport,
  type FeatureFlagResult,
  type JsonType,
} from './analytics.types';
