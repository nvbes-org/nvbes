import type {
  FeatureFlagResult,
  JsonType,
  PostHogConfig,
} from 'posthog-js/dist/module.full.no-external';
import {
  ALLOWED_EVENTS,
  DEFAULT_BLOCKED_ROUTE_PATTERNS,
  clearPostHogStorage,
  hasAnyPostHogConsent,
  isUuidLike,
  normalizePostHogError,
  sanitizePostHogProperties,
} from './posthog.privacy';
import type {
  PostHogPurposeConsent,
  PostHogRuntime,
  PostHogRuntimeOptions,
} from './posthog.types';

type PostHogModule = typeof import('posthog-js/dist/module.full.no-external');
type PostHogClient = PostHogModule['default'];

export class RuntimePostHog implements PostHogRuntime {
  private client: PostHogClient | null = null;
  private initPromise: Promise<PostHogClient | null> | null = null;
  private unsubscribeConsent: (() => void) | null = null;
  private consent: PostHogPurposeConsent;
  private readonly blockedRoutePatterns: RegExp[];

  constructor(private readonly options: PostHogRuntimeOptions) {
    this.consent = options.getConsent();
    this.blockedRoutePatterns = [
      ...DEFAULT_BLOCKED_ROUTE_PATTERNS,
      ...(options.blockedRoutePatterns ?? []),
    ];
  }

  async init(): Promise<void> {
    if (!this.unsubscribeConsent && this.options.onConsentChange) {
      this.unsubscribeConsent = this.options.onConsentChange((consent) => {
        void this.applyConsent(consent);
      });
    }

    if (!hasAnyPostHogConsent(this.consent)) {
      await this.disableCapture();
      return;
    }

    await this.ensureClient();
  }

  async applyConsent(consent = this.options.getConsent()): Promise<void> {
    this.consent = consent;
    if (!hasAnyPostHogConsent(consent)) {
      await this.disableCapture();
      return;
    }

    const client = await this.ensureClient();
    if (!client) return;

    client.opt_in_capturing();
    client.set_config(this.runtimeConfig(consent));

    if (this.canCaptureReplay()) {
      client.startSessionRecording({ sampling: true });
    } else {
      client.stopSessionRecording();
    }
  }

  async trackProductEvent(name: string, properties?: Record<string, unknown>): Promise<void> {
    if (!this.consent.productAnalytics || !ALLOWED_EVENTS.has(name)) {
      return;
    }

    const client = await this.ensureClient();
    if (!client) return;

    const sanitized = await sanitizePostHogProperties(
      {
        ...this.options.getCommonProperties?.(),
        ...properties,
        app_name: this.options.appName,
        route_path: this.routePath(),
      },
      (prefix, id) => this.pseudonymousId(prefix, id),
    );

    client.capture(name, sanitized);
  }

  async identifyProductUser(userId: string, traits?: Record<string, unknown>): Promise<void> {
    if (!this.consent.productAnalytics || !isUuidLike(userId)) {
      return;
    }

    const client = await this.ensureClient();
    if (!client) return;

    const distinctId = await this.pseudonymousId('usr', userId);
    client.identify(distinctId, await this.sanitizeProperties(traits ?? {}));
  }

  async setPostHogWorkspaceGroup(
    workspaceId: string,
    traits?: Record<string, unknown>,
  ): Promise<void> {
    if (!this.consent.productAnalytics || !isUuidLike(workspaceId)) {
      return;
    }

    const client = await this.ensureClient();
    if (!client) return;

    const groupId = await this.pseudonymousId('wks', workspaceId);
    client.group('workspace', groupId, await this.sanitizeProperties(traits ?? {}));
  }

  async getFeatureFlag(key: string): Promise<FeatureFlagResult | undefined> {
    if (!this.consent.featureFlags || this.isSensitiveRoute()) {
      return undefined;
    }

    const client = await this.ensureClient();
    return client?.getFeatureFlagResult(key);
  }

  async getFeatureFlagPayload(key: string): Promise<JsonType | undefined> {
    if (!this.consent.featureFlags || this.isSensitiveRoute()) {
      return undefined;
    }

    const client = await this.ensureClient();
    return client?.getFeatureFlagPayload(key);
  }

  async isFeatureEnabled(key: string): Promise<boolean> {
    if (!this.consent.featureFlags || this.isSensitiveRoute()) {
      return false;
    }

    const client = await this.ensureClient();
    return client?.isFeatureEnabled(key) === true;
  }

  async trackExperimentExposure(key: string, variant: string): Promise<void> {
    if (!this.consent.featureFlags) {
      return;
    }

    await this.trackProductEvent('posthog.experiment_exposure', {
      feature_flag: key,
      variant,
    });
  }

  async capturePostHogException(
    error: unknown,
    properties?: Record<string, unknown>,
  ): Promise<void> {
    if (!this.consent.errorTracking || this.isSensitiveRoute()) {
      return;
    }

    const client = await this.ensureClient();
    if (!client) return;

    const normalized = normalizePostHogError(error);
    client.captureException(error, {
      ...(await this.sanitizeProperties({
        ...properties,
        error_name: normalized.name,
        error_message: normalized.message,
        app_name: this.options.appName,
        route_path: this.routePath(),
      })),
    });
  }

  async startPrivacySafeReplay(): Promise<void> {
    if (!this.canCaptureReplay()) {
      return;
    }

    const client = await this.ensureClient();
    client?.startSessionRecording({ sampling: true });
  }

  async stopPrivacySafeReplay(): Promise<void> {
    const client = await this.ensureClient();
    client?.stopSessionRecording();
  }

  private async ensureClient(): Promise<PostHogClient | null> {
    if (this.client) {
      return this.client;
    }

    if (!this.options.apiKey) {
      return null;
    }

    if (!this.initPromise) {
      this.initPromise = import('posthog-js/dist/module.full.no-external')
        .then((module) => {
          const client = module.default;
          client.init(this.options.apiKey ?? '', this.runtimeConfig(this.consent));
          client.opt_out_capturing();
          this.client = client;
          return client;
        })
        .catch(() => null);
    }

    const client = await this.initPromise;
    if (client && hasAnyPostHogConsent(this.consent)) {
      client.opt_in_capturing();
    }
    return client;
  }

  private runtimeConfig(consent: PostHogPurposeConsent): Partial<PostHogConfig> {
    const routeBlocked = this.isSensitiveRoute();
    return {
      api_host: this.options.apiHost || 'https://eu.i.posthog.com',
      defaults: '2026-01-30',
      person_profiles: 'identified_only',
      persistence: 'localStorage',
      opt_out_capturing_by_default: true,
      capture_pageview: false,
      capture_pageleave: consent.productAnalytics && !routeBlocked,
      autocapture: consent.autocaptureHeatmaps && !routeBlocked,
      disable_session_recording: !consent.sessionReplay || routeBlocked,
      capture_exceptions: consent.errorTracking && !routeBlocked,
      disable_surveys: !consent.surveysFeedback || routeBlocked,
      disable_surveys_automatic_display: !consent.surveysFeedback || routeBlocked,
      advanced_disable_feature_flags: !consent.featureFlags || routeBlocked,
      advanced_disable_feature_flags_on_first_load: !consent.featureFlags || routeBlocked,
      mask_all_text: true,
      mask_all_element_attributes: true,
      session_recording: {
        maskAllInputs: true,
        maskTextSelector: '*',
        blockSelector:
          '[data-posthog-block], input, textarea, select, [contenteditable="true"], [data-sensitive="true"]',
      },
      error_tracking: {
        captureExtensionExceptions: false,
      },
      surveys: consent.surveysFeedback && !routeBlocked ? { prefillFromUrl: false } : undefined,
      before_send: (event) => {
        if (!event) return null;
        if (this.isSensitiveRoute()) return null;
        if (typeof event.event === 'string' && !this.allowEventName(event.event)) return null;
        return event;
      },
    };
  }

  private async disableCapture(): Promise<void> {
    const client = this.client;
    if (client) {
      client.stopSessionRecording();
      client.stopExceptionAutocapture();
      client.opt_out_capturing();
      client.reset();
    }

    clearPostHogStorage();
  }

  private canCaptureReplay(): boolean {
    return this.consent.sessionReplay && !this.isSensitiveRoute();
  }

  private routePath(): string {
    if (this.options.getRoutePath) {
      return this.options.getRoutePath();
    }
    if (typeof window === 'undefined') {
      return '/';
    }
    return window.location.pathname || '/';
  }

  private isSensitiveRoute(): boolean {
    const route = this.routePath();
    return this.blockedRoutePatterns.some((pattern) => pattern.test(route));
  }

  private allowEventName(name: string): boolean {
    if (ALLOWED_EVENTS.has(name)) return true;
    return name.startsWith('$') && this.consent.autocaptureHeatmaps && !this.isSensitiveRoute();
  }

  private sanitizeProperties(properties: Record<string, unknown>) {
    return sanitizePostHogProperties(properties, (prefix, id) => this.pseudonymousId(prefix, id));
  }

  private async pseudonymousId(prefix: string, id: string): Promise<string> {
    const salt = this.options.analyticsSalt || `${this.options.appName}:analytics`;
    const key = await crypto.subtle.importKey(
      'raw',
      new TextEncoder().encode(salt),
      { name: 'HMAC', hash: 'SHA-256' },
      false,
      ['sign'],
    );
    const signature = await crypto.subtle.sign('HMAC', key, new TextEncoder().encode(id));
    return `${prefix}_${hex(signature).slice(0, 32)}`;
  }
}

function hex(buffer: ArrayBuffer): string {
  return Array.from(new Uint8Array(buffer))
    .map((byte) => byte.toString(16).padStart(2, '0'))
    .join('');
}
