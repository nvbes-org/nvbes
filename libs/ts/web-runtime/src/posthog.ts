import type { FeatureFlagResult, JsonType } from 'posthog-js/dist/module.full.no-external';
import { hasAnyPostHogConsent } from './posthog.privacy';
import { RuntimePostHog } from './posthog.runtime';
import type { PostHogRuntime, PostHogRuntimeOptions } from './posthog.types';

let runtimeSingleton: RuntimePostHog | null = null;

export function initPostHogRuntime(options: PostHogRuntimeOptions): PostHogRuntime {
  runtimeSingleton = new RuntimePostHog(options);
  void runtimeSingleton.init();
  return runtimeSingleton;
}

export function getPostHogRuntime(): PostHogRuntime | null {
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

export async function setPostHogWorkspaceGroup(
  workspaceId: string,
  traits?: Record<string, unknown>,
): Promise<void> {
  await runtimeSingleton?.setPostHogWorkspaceGroup(workspaceId, traits);
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

export async function capturePostHogException(
  error: unknown,
  properties?: Record<string, unknown>,
): Promise<void> {
  await runtimeSingleton?.capturePostHogException(error, properties);
}

export async function startPrivacySafeReplay(): Promise<void> {
  await runtimeSingleton?.startPrivacySafeReplay();
}

export async function stopPrivacySafeReplay(): Promise<void> {
  await runtimeSingleton?.stopPrivacySafeReplay();
}

export { hasAnyPostHogConsent };
export {
  ALL_POSTHOG_CONSENT,
  EMPTY_POSTHOG_CONSENT,
  type PostHogPurposeConsent,
  type PostHogRuntime,
  type PostHogRuntimeOptions,
} from './posthog.types';
