import {
  ALLOWED_EVENTS,
  clearAnalyticsStorage,
  DEFAULT_BLOCKED_ROUTE_PATTERNS,
  hasAnyAnalyticsConsent,
  isUuidLike,
  normalizeAnalyticsError,
  sanitizeAnalyticsProperties,
} from './analytics.privacy';
import type {
  AnalyticsPurposeConsent,
  AnalyticsRuntime,
  AnalyticsRuntimeOptions,
} from './analytics.types';

export class RuntimeAnalytics implements AnalyticsRuntime {
  private unsubscribeConsent: (() => void) | null = null;
  private consent: AnalyticsPurposeConsent;
  private readonly blockedRoutePatterns: RegExp[];

  constructor(private readonly options: AnalyticsRuntimeOptions) {
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

    await this.options.transport?.setProductAnalyticsEnabled?.(
      hasProductAnalyticsConsent(this.consent),
    );
    await this.options.transport?.setErrorReportingEnabled?.(this.consent.errorTracking);

    if (!hasAnyAnalyticsConsent(this.consent)) {
      await this.disableCapture();
    }
  }

  async applyConsent(consent = this.options.getConsent()): Promise<void> {
    this.consent = consent;
    await this.options.transport?.setProductAnalyticsEnabled?.(hasProductAnalyticsConsent(consent));
    await this.options.transport?.setErrorReportingEnabled?.(consent.errorTracking);
    if (!hasAnyAnalyticsConsent(consent)) {
      await this.disableCapture();
      return;
    }

    if (!this.canCaptureReplay()) {
      await this.options.transport?.stopPrivacySafeReplay?.();
    }
  }

  async trackProductEvent(name: string, properties?: Record<string, unknown>): Promise<void> {
    if (!this.consent.productAnalytics || !ALLOWED_EVENTS.has(name)) {
      return;
    }

    const sanitized = await sanitizeAnalyticsProperties(
      {
        ...this.options.getCommonProperties?.(),
        ...properties,
        app_name: this.options.appName,
        route_path: this.routePath(),
      },
      (prefix, id) => this.pseudonymousId(prefix, id),
    );

    await this.options.transport?.trackProductEvent?.(name, sanitized);
  }

  async identifyProductUser(userId: string, traits?: Record<string, unknown>): Promise<void> {
    if (!this.consent.productAnalytics || !isUuidLike(userId)) {
      return;
    }

    const distinctId = await this.pseudonymousId('usr', userId);
    const sanitized = await this.sanitizeProperties(traits ?? {});
    await this.options.transport?.identifyProductUser?.(distinctId, sanitized);
  }

  async setAnalyticsWorkspaceGroup(
    workspaceId: string,
    traits?: Record<string, unknown>,
  ): Promise<void> {
    if (!this.consent.productAnalytics || !isUuidLike(workspaceId)) {
      return;
    }

    const groupId = await this.pseudonymousId('wks', workspaceId);
    const sanitized = await this.sanitizeProperties(traits ?? {});
    await this.options.transport?.setWorkspaceGroup?.(groupId, sanitized);
  }

  async getFeatureFlag(key: string) {
    if (!this.consent.featureFlags || this.isSensitiveRoute()) {
      return undefined;
    }

    return this.options.transport?.getFeatureFlag?.(key);
  }

  async getFeatureFlagPayload(key: string) {
    if (!this.consent.featureFlags || this.isSensitiveRoute()) {
      return undefined;
    }

    return this.options.transport?.getFeatureFlagPayload?.(key);
  }

  async isFeatureEnabled(key: string): Promise<boolean> {
    if (!this.consent.featureFlags || this.isSensitiveRoute()) {
      return false;
    }

    return (await this.options.transport?.getFeatureFlag?.(key)) === true;
  }

  async trackExperimentExposure(key: string, variant: string): Promise<void> {
    if (!this.consent.featureFlags) {
      return;
    }

    await this.trackProductEvent('analytics.experiment_exposure', {
      feature_flag: key,
      variant,
    });
  }

  async captureAnalyticsException(
    error: unknown,
    properties?: Record<string, unknown>,
  ): Promise<void> {
    if (!this.consent.errorTracking || this.isSensitiveRoute()) {
      return;
    }

    const normalized = normalizeAnalyticsError(error);
    const sanitized = await this.sanitizeProperties({
      ...properties,
      error_name: normalized.name,
      error_message: normalized.message,
      app_name: this.options.appName,
      route_path: this.routePath(),
    });

    await this.options.transport?.captureException?.(error, sanitized);
  }

  async startPrivacySafeReplay(): Promise<void> {
    if (!this.canCaptureReplay()) {
      return;
    }

    await this.options.transport?.startPrivacySafeReplay?.();
  }

  async stopPrivacySafeReplay(): Promise<void> {
    await this.options.transport?.stopPrivacySafeReplay?.();
  }

  private async disableCapture(): Promise<void> {
    if (this.options.transport?.disableCapture) {
      await this.options.transport.disableCapture();
    } else {
      await this.options.transport?.stopPrivacySafeReplay?.();
    }
    clearAnalyticsStorage();
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

  private sanitizeProperties(properties: Record<string, unknown>) {
    return sanitizeAnalyticsProperties(properties, (prefix, id) => this.pseudonymousId(prefix, id));
  }

  private async pseudonymousId(prefix: string, id: string): Promise<string> {
    if (!globalThis.crypto?.subtle) {
      return `${prefix}_unsupported`;
    }

    const salt = this.options.analyticsSalt || `${this.options.appName}:analytics`;
    const key = await globalThis.crypto.subtle.importKey(
      'raw',
      new TextEncoder().encode(salt),
      { name: 'HMAC', hash: 'SHA-256' },
      false,
      ['sign'],
    );
    const signature = await globalThis.crypto.subtle.sign(
      'HMAC',
      key,
      new TextEncoder().encode(id),
    );
    return `${prefix}_${hex(signature).slice(0, 32)}`;
  }
}

function hasProductAnalyticsConsent(consent: AnalyticsPurposeConsent): boolean {
  return (
    consent.productAnalytics ||
    consent.autocaptureHeatmaps ||
    consent.sessionReplay ||
    consent.surveysFeedback ||
    consent.featureFlags
  );
}

function hex(buffer: ArrayBuffer): string {
  return Array.from(new Uint8Array(buffer))
    .map((byte) => byte.toString(16).padStart(2, '0'))
    .join('');
}
