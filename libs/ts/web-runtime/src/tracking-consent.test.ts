import { afterEach, describe, expect, it } from 'vite-plus/test';
import { HttpError } from '@nvbes/http-client';
import {
  ACCEPT_ALL_CONSENT,
  DECLINE_ALL_CONSENT,
  createTrackingConsentApi,
} from './tracking-consent';

const STORAGE_KEY_V3 = 'nvbes.tracking-consent.v3';

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
    STORAGE_KEY_V3,
    JSON.stringify({
      version: 3,
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

  it('syncs local browser consent once the session becomes authenticated', async () => {
    installTestWindow();
    const localSavedAt = new Date(Date.now() - 60_000).toISOString();
    installStoredConsent(ACCEPT_ALL_CONSENT, localSavedAt);

    let authenticated = false;
    let listConsentsCalls = 0;
    let grantConsentCalls = 0;

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
    expect(grantConsentCalls).toBeGreaterThan(0);
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
            document_version: 'v3',
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

    const stored = JSON.parse(window.localStorage.getItem(STORAGE_KEY_V3) ?? '{}') as {
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
