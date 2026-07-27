export type FeatureFlagResult = boolean | string | number | null;

export type JsonType = string | number | boolean | null | JsonType[] | { [key: string]: JsonType };

export type AnalyticsProperties = Record<string, string | number | boolean | null>;

export interface AnalyticsCorrelationContext {
  distinctId: string;
  sessionId: string;
}

export interface AnalyticsPurposeConsent {
  productAnalytics: boolean;
  autocaptureHeatmaps: boolean;
  sessionReplay: boolean;
  surveysFeedback: boolean;
  errorTracking: boolean;
  featureFlags: boolean;
}

export interface AnalyticsTransport {
  trackProductEvent?: (name: string, properties: AnalyticsProperties) => Promise<void> | void;
  identifyProductUser?: (userId: string, traits: AnalyticsProperties) => Promise<void> | void;
  setWorkspaceGroup?: (workspaceId: string, traits: AnalyticsProperties) => Promise<void> | void;
  getFeatureFlag?: (
    key: string,
  ) => Promise<FeatureFlagResult | undefined> | FeatureFlagResult | undefined;
  getFeatureFlagPayload?: (key: string) => Promise<JsonType | undefined> | JsonType | undefined;
  getCorrelationContext?: () =>
    | Promise<AnalyticsCorrelationContext | undefined>
    | AnalyticsCorrelationContext
    | undefined;
  captureException?: (error: unknown, properties: AnalyticsProperties) => Promise<void> | void;
  startPrivacySafeReplay?: () => Promise<void> | void;
  stopPrivacySafeReplay?: () => Promise<void> | void;
  setProductAnalyticsEnabled?: (enabled: boolean) => Promise<void> | void;
  setErrorReportingEnabled?: (enabled: boolean) => Promise<void> | void;
  disableCapture?: () => Promise<void> | void;
}

export interface AnalyticsRuntimeOptions {
  appName: string;
  analyticsSalt?: string;
  getConsent: () => AnalyticsPurposeConsent;
  onConsentChange?: (listener: (consent: AnalyticsPurposeConsent) => void) => () => void;
  getRoutePath?: () => string;
  getCommonProperties?: () => Record<string, unknown>;
  blockedRoutePatterns?: RegExp[];
  transport?: AnalyticsTransport;
}

export interface AnalyticsRuntime {
  init(): Promise<void>;
  applyConsent(consent?: AnalyticsPurposeConsent): Promise<void>;
  trackProductEvent(name: string, properties?: Record<string, unknown>): Promise<void>;
  identifyProductUser(userId: string, traits?: Record<string, unknown>): Promise<void>;
  setAnalyticsWorkspaceGroup(workspaceId: string, traits?: Record<string, unknown>): Promise<void>;
  getFeatureFlag(key: string): Promise<FeatureFlagResult | undefined>;
  getFeatureFlagPayload(key: string): Promise<JsonType | undefined>;
  isFeatureEnabled(key: string): Promise<boolean>;
  trackExperimentExposure(key: string, variant: string): Promise<void>;
  getCorrelationContext(): Promise<AnalyticsCorrelationContext | undefined>;
  captureAnalyticsException(error: unknown, properties?: Record<string, unknown>): Promise<void>;
  startPrivacySafeReplay(): Promise<void>;
  stopPrivacySafeReplay(): Promise<void>;
}

export const EMPTY_ANALYTICS_CONSENT: AnalyticsPurposeConsent = {
  productAnalytics: false,
  autocaptureHeatmaps: false,
  sessionReplay: false,
  surveysFeedback: false,
  errorTracking: false,
  featureFlags: false,
};

export const ALL_ANALYTICS_CONSENT: AnalyticsPurposeConsent = {
  productAnalytics: true,
  autocaptureHeatmaps: true,
  sessionReplay: true,
  surveysFeedback: true,
  errorTracking: true,
  featureFlags: true,
};
