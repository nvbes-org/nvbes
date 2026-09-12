import { HttpError } from '@nvbes/http-client';
import { afterEach, describe, expect, it } from 'vite-plus/test';
import {
  ACCEPT_ALL_CONSENT,
  createTrackingConsentApi,
  DECLINE_ALL_CONSENT,
} from './tracking-consent';

const STORAGE_KEY_V4 = 'nvbes.tracking-consent.v4';
const NOTICE_VERSION = 'cookie-notice-2026-07-20';

type MemoryStorage = {
  getItem: (key: string) => string | null;
  setItem: (key: string, value: string) => void;
  removeItem: (key: string) => void;
};

function createMemoryStorage(): MemoryStorage {
  const values = new Map<string, string>();

  return {
    getItem: (key) => values.get(key) ?? null,
    setItem: (key, value) => {
      values.set(key, value);
    },
    removeItem: (key) => {
      values.delete(key);
    },
  };
}

function installTestWindow() {
  const localStorage = createMemoryStorage();

  Object.defineProperty(globalThis, 'window', {
    configurable: true,
    value: {
      localStorage,
      dispatchEvent: () => true,
    },
    writable: true,
  });

  return localStorage;
}

function installStoredConsent(consent = ACCEPT_ALL_CONSENT, savedAt = new Date().toISOString()) {
  const expiresAt = new Date(Date.parse(savedAt) + 1_000_000).toISOString();
  window.localStorage.setItem(
    STORAGE_KEY_V4,
    JSON.stringify({
      version: 4,
      savedAt,
      expiresAt,
      source: 'test',
      consent,
    }),
  );
}

afterEach(() => {
  Reflect.deleteProperty(globalThis, 'window');
});

describe('tracking consent sync', () => {
  it('accepts only purposes implemented by the current products', () => {
    expect(ACCEPT_ALL_CONSENT.analytics).toEqual({
      productAnalytics: true,
      autocaptureHeatmaps: false,
      sessionReplay: false,
      surveysFeedback: false,
      errorTracking: true,
      featureFlags: false,
    });
  });

  it('skips backend sync when the session is not authenticated', async () => {
    installTestWindow();
    installStoredConsent();

    let listConsentsCalls = 0;
    let grantConsentCalls = 0;
    let revokeConsentCalls = 0;

    const api = createTrackingConsentApi({
      identityClient: {
        listConsents: async () => {
          listConsentsCalls += 1;
          return [];
        },
        grantConsent: async () => {
          grantConsentCalls += 1;
          return {};
        },
        revokeConsent: async () => {
          revokeConsentCalls += 1;
          return {};
        },
        isAuthenticated: async () => false,
      },
      defaultSource: 'test',
    });

    await api.syncTrackingConsent();

    expect(listConsentsCalls).toBe(0);
    expect(grantConsentCalls).toBe(0);
    expect(revokeConsentCalls).toBe(0);
  });

  it('suspends optional tracking before a subject switch without mutating the previous account', () => {
    installTestWindow();
    installStoredConsent(ACCEPT_ALL_CONSENT);

    let listConsentsCalls = 0;
    let mutationCalls = 0;
    const api = createTrackingConsentApi({
      identityClient: {
        listConsents: async () => {
          listConsentsCalls += 1;
          return [];
        },
        grantConsent: async () => {
          mutationCalls += 1;
          return {};
        },
        revokeConsent: async () => {
          mutationCalls += 1;
          return {};
        },
      },
      defaultSource: 'test',
    });

    api.prepareTrackingConsentSubjectSwitch('test:subject-switch');

    expect(api.getTrackingConsent()).toEqual(DECLINE_ALL_CONSENT);
    expect(listConsentsCalls).toBe(0);
    expect(mutationCalls).toBe(0);
    const stored = JSON.parse(window.localStorage.getItem(STORAGE_KEY_V4) ?? '{}') as {
      source?: string;
    };
    expect(stored.source).toBe('test:subject-switch');
  });

  it('reactivates only the target account backend consent after a subject switch', async () => {
    installTestWindow();
    installStoredConsent(ACCEPT_ALL_CONSENT);

    let mutationCalls = 0;
    const grantedAt = new Date().toISOString();
    const api = createTrackingConsentApi({
      identityClient: {
        listConsents: async () => [
          {
            consent_type: 'analytics_product_analytics',
            document_version: NOTICE_VERSION,
            granted_at: grantedAt,
            revoked_at: null,
          },
        ],
        grantConsent: async () => {
          mutationCalls += 1;
          return {};
        },
        revokeConsent: async () => {
          mutationCalls += 1;
          return {};
        },
        isAuthenticated: async () => true,
      },
      defaultSource: 'test',
    });

    api.prepareTrackingConsentSubjectSwitch();
    expect(api.getAnalyticsConsent().productAnalytics).toBe(false);

    await api.syncTrackingConsent({ sourceOfTruth: 'backend' });

    expect(api.getAnalyticsConsent().productAnalytics).toBe(true);
    expect(api.getTrackingConsent()?.vendors.posthog).toBe(true);
    expect(api.getAnalyticsConsent().errorTracking).toBe(false);
    expect(mutationCalls).toBe(0);
  });

  it('keeps tracking disabled when the target account has no optional consent', async () => {
    installTestWindow();
    installStoredConsent(ACCEPT_ALL_CONSENT);

    let mutationCalls = 0;
    const api = createTrackingConsentApi({
      identityClient: {
        listConsents: async () => [],
        grantConsent: async () => {
          mutationCalls += 1;
          return {};
        },
        revokeConsent: async () => {
          mutationCalls += 1;
          return {};
        },
        isAuthenticated: async () => true,
      },
      defaultSource: 'test',
    });

    api.prepareTrackingConsentSubjectSwitch();
    await api.syncTrackingConsent({ sourceOfTruth: 'backend' });

    expect(api.getTrackingConsent()).toEqual(DECLINE_ALL_CONSENT);
    expect(mutationCalls).toBe(0);
  });

  it('syncs local browser consent once the session becomes authenticated', async () => {
    installTestWindow();
    const localSavedAt = new Date(Date.now() - 60_000).toISOString();
    installStoredConsent(ACCEPT_ALL_CONSENT, localSavedAt);

    let authenticated = false;
    let listConsentsCalls = 0;
    const grantedConsentTypes: string[] = [];

    const api = createTrackingConsentApi({
      identityClient: {
        listConsents: async () => {
          listConsentsCalls += 1;
          return [];
        },
        grantConsent: async (consentType) => {
          grantedConsentTypes.push(consentType);
          return {};
        },
        revokeConsent: async () => ({}),
        isAuthenticated: async () => authenticated,
      },
      defaultSource: 'test',
    });

    await api.syncTrackingConsent();
    expect(listConsentsCalls).toBe(0);

    authenticated = true;
    await api.syncTrackingConsent();

    expect(listConsentsCalls).toBe(1);
    expect(grantedConsentTypes).toContain('cookie_consent_analytics');
    expect(grantedConsentTypes).toContain('cookie_consent_vendor_posthog');
    expect(grantedConsentTypes).toContain('cookie_consent_vendor_sentry');
    expect(grantedConsentTypes).toContain('cookie_consent_vendor_grafana');
    expect(grantedConsentTypes).toContain('analytics_product_analytics');
    expect(grantedConsentTypes).toContain('analytics_error_tracking');
    expect(grantedConsentTypes).not.toContain('analytics_autocapture_heatmaps');
    expect(grantedConsentTypes).not.toContain('analytics_session_replay');
    expect(grantedConsentTypes).not.toContain('analytics_surveys_feedback');
    expect(grantedConsentTypes).not.toContain('analytics_feature_flags');
    expect(grantedConsentTypes).not.toContain('cookie_consent_essentials');
    expect(grantedConsentTypes).not.toContain('cookie_consent_vendor_stripe');
    expect(grantedConsentTypes).not.toContain('cookie_consent_vendor_identity');
    expect(grantedConsentTypes).not.toContain('cookie_consent_vendor_cloudflare');
  });

  it('revokes unavailable purpose consent rows during synchronization', async () => {
    installTestWindow();
    installStoredConsent(ACCEPT_ALL_CONSENT);

    const revokedConsentTypes: string[] = [];

    const api = createTrackingConsentApi({
      identityClient: {
        listConsents: async () => [
          {
            consent_type: 'analytics_session_replay',
            document_version: NOTICE_VERSION,
            granted_at: new Date(Date.now() - 60_000).toISOString(),
            revoked_at: null,
          },
          {
            consent_type: 'cookie_consent_vendor_analytics',
            document_version: NOTICE_VERSION,
            granted_at: new Date(Date.now() - 60_000).toISOString(),
            revoked_at: null,
          },
        ],
        grantConsent: async () => ({}),
        revokeConsent: async (consentType) => {
          revokedConsentTypes.push(consentType);
          return {};
        },
        isAuthenticated: async () => true,
      },
      defaultSource: 'test',
    });

    await api.syncTrackingConsent();

    expect(revokedConsentTypes).toContain('analytics_session_replay');
    expect(revokedConsentTypes).toContain('cookie_consent_vendor_analytics');
  });

  it('restores only the purposes explicitly granted by the backend', async () => {
    installTestWindow();

    const grantedAt = new Date().toISOString();
    const api = createTrackingConsentApi({
      identityClient: {
        listConsents: async () => [
          {
            consent_type: 'analytics_product_analytics',
            document_version: NOTICE_VERSION,
            granted_at: grantedAt,
            revoked_at: null,
          },
        ],
        grantConsent: async () => ({}),
        revokeConsent: async () => ({}),
        isAuthenticated: async () => true,
      },
      defaultSource: 'test',
    });

    await api.syncTrackingConsent();

    expect(api.getAnalyticsConsent()).toEqual({
      productAnalytics: true,
      autocaptureHeatmaps: false,
      sessionReplay: false,
      surveysFeedback: false,
      errorTracking: false,
      featureFlags: false,
    });
  });

  it('requires a new choice after a legacy binary acceptance', () => {
    installTestWindow();
    window.localStorage.setItem('nvbes.tracking-consent.v1', 'accepted');

    const api = createTrackingConsentApi({
      identityClient: {
        listConsents: async () => [],
        grantConsent: async () => ({}),
        revokeConsent: async () => ({}),
      },
      defaultSource: 'test',
    });

    expect(api.getTrackingConsent()).toBeNull();
  });

  it('treats explicit current purposes as authoritative over stale categories', () => {
    installTestWindow();
    installStoredConsent({
      ...ACCEPT_ALL_CONSENT,
      analytics: { ...DECLINE_ALL_CONSENT.analytics },
    });

    const api = createTrackingConsentApi({
      identityClient: {
        listConsents: async () => [],
        grantConsent: async () => ({}),
        revokeConsent: async () => ({}),
      },
      defaultSource: 'test',
    });

    expect(api.getAnalyticsConsent()).toEqual(DECLINE_ALL_CONSENT.analytics);
  });

  it('does not restore consent from an obsolete backend notice version', async () => {
    installTestWindow();
    const api = createTrackingConsentApi({
      identityClient: {
        listConsents: async () => [
          {
            consent_type: 'analytics_product_analytics',
            document_version: 'v3',
            granted_at: new Date().toISOString(),
            revoked_at: null,
          },
        ],
        grantConsent: async () => ({}),
        revokeConsent: async () => ({}),
        isAuthenticated: async () => true,
      },
      defaultSource: 'test',
    });

    await api.syncTrackingConsent();

    expect(api.getTrackingConsent()).toBeNull();
  });

  it('updates the front when the backend consent change is newer', async () => {
    installTestWindow();
    const localSavedAt = new Date(Date.now() - 60_000).toISOString();
    installStoredConsent(ACCEPT_ALL_CONSENT, localSavedAt);

    const backendRevokedAt = new Date(Date.now() + 60_000).toISOString();
    let grantConsentCalls = 0;
    let revokeConsentCalls = 0;

    const api = createTrackingConsentApi({
      identityClient: {
        listConsents: async () => [
          {
            consent_type: 'cookie_consent_analytics',
            document_version: NOTICE_VERSION,
            granted_at: localSavedAt,
            revoked_at: backendRevokedAt,
          },
        ],
        grantConsent: async () => {
          grantConsentCalls += 1;
          return {};
        },
        revokeConsent: async () => {
          revokeConsentCalls += 1;
          return {};
        },
        isAuthenticated: async () => true,
      },
      defaultSource: 'test',
    });

    await api.syncTrackingConsent();

    expect(grantConsentCalls).toBe(0);
    expect(revokeConsentCalls).toBe(0);
    expect(api.getTrackingConsent()).toEqual(DECLINE_ALL_CONSENT);

    const stored = JSON.parse(window.localStorage.getItem(STORAGE_KEY_V4) ?? '{}') as {
      savedAt?: string;
      consent?: unknown;
    };
    expect(stored.savedAt).toBe(backendRevokedAt);
  });

  it('ignores consent sync when the backend returns unauthorized', async () => {
    installTestWindow();
    installStoredConsent();

    const response = new Response(JSON.stringify({ error: { message: 'Unauthorized' } }), {
      status: 401,
      statusText: 'Unauthorized',
      headers: { 'Content-Type': 'application/json' },
    });

    const api = createTrackingConsentApi({
      identityClient: {
        listConsents: async () => {
          throw new HttpError('Failed to list consents: Unauthorized', response, {
            error: { message: 'Unauthorized' },
          });
        },
        grantConsent: async () => ({}),
        revokeConsent: async () => ({}),
        isAuthenticated: async () => true,
      },
      defaultSource: 'test',
    });

    await expect(api.syncTrackingConsent()).resolves.toBeUndefined();
    expect(api.getTrackingConsent()).toEqual(ACCEPT_ALL_CONSENT);
  });
});
