import type {
  FeatureFlagResult,
  JsonType,
} from 'posthog-js/dist/module.full.no-external';

export interface PostHogPurposeConsent {
  productAnalytics: boolean;
  autocaptureHeatmaps: boolean;
  sessionReplay: boolean;
  surveysFeedback: boolean;
  errorTracking: boolean;
  featureFlags: boolean;
}

export interface PostHogRuntimeOptions {
  appName: string;
  apiKey?: string;
  apiHost?: string;
  analyticsSalt?: string;
  getConsent: () => PostHogPurposeConsent;
  onConsentChange?: (listener: (consent: PostHogPurposeConsent) => void) => () => void;
  getRoutePath?: () => string;
  getCommonProperties?: () => Record<string, unknown>;
  blockedRoutePatterns?: RegExp[];
}

export interface PostHogRuntime {
  init(): Promise<void>;
  applyConsent(consent?: PostHogPurposeConsent): Promise<void>;
  trackProductEvent(name: string, properties?: Record<string, unknown>): Promise<void>;
  identifyProductUser(userId: string, traits?: Record<string, unknown>): Promise<void>;
  setPostHogWorkspaceGroup(workspaceId: string, traits?: Record<string, unknown>): Promise<void>;
  getFeatureFlag(key: string): Promise<FeatureFlagResult | undefined>;
  getFeatureFlagPayload(key: string): Promise<JsonType | undefined>;
  isFeatureEnabled(key: string): Promise<boolean>;
  trackExperimentExposure(key: string, variant: string): Promise<void>;
  capturePostHogException(error: unknown, properties?: Record<string, unknown>): Promise<void>;
  startPrivacySafeReplay(): Promise<void>;
  stopPrivacySafeReplay(): Promise<void>;
}

export const EMPTY_POSTHOG_CONSENT: PostHogPurposeConsent = {
  productAnalytics: false,
  autocaptureHeatmaps: false,
  sessionReplay: false,
  surveysFeedback: false,
  errorTracking: false,
  featureFlags: false,
};

export const ALL_POSTHOG_CONSENT: PostHogPurposeConsent = {
  productAnalytics: true,
  autocaptureHeatmaps: true,
  sessionReplay: true,
  surveysFeedback: true,
  errorTracking: true,
  featureFlags: true,
};
