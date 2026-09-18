import { afterEach, beforeEach, describe, expect, it, vi } from 'vite-plus/test';
import {
  ACCEPT_ALL_CONSENT,
  DECLINE_ALL_CONSENT,
  cloneConsent,
  getAnalyticsConsent,
  getTrackingConsent,
  hasAnyOptionalConsent,
  isAnalyticsPurposeAccepted,
  isCategoryAccepted,
  isVendorAccepted,
  persistTrackingConsent,
  readTrackingConsentStoredValue,
} from './tracking-consent.storage';

const key = 'nvbes.tracking-consent.v4';
const now = new Date('2026-09-11T00:00:00.000Z');
const values = new Map<string, string>();
const valid = () => ({
  version: 4,
  source: 'synthetic-test',
  savedAt: now.toISOString(),
  expiresAt: '2026-09-12T00:00:00.000Z',
  consent: ACCEPT_ALL_CONSENT,
});

beforeEach(() => {
  vi.useFakeTimers();
  vi.setSystemTime(now);
  values.clear();
  vi.stubGlobal('window', {
    localStorage: {
      getItem: (name: string) => values.get(name) ?? null,
      setItem: (name: string, value: string) => values.set(name, value),
      removeItem: (name: string) => values.delete(name),
    },
  });
});
afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

describe('stored consent validation', () => {
  it.each([
    null,
    [],
    {},
    'accepted',
    { ...valid(), version: 3 },
    { ...valid(), source: '' },
    { ...valid(), savedAt: 'invalid' },
    { ...valid(), expiresAt: undefined },
    { ...valid(), expiresAt: 'invalid' },
    { ...valid(), expiresAt: now.toISOString() },
    { ...valid(), savedAt: '2026-09-13T00:00:00.000Z' },
    { ...valid(), expiresAt: '2027-09-12T00:00:00.000Z' },
    { ...valid(), consent: { categories: { analytics: true }, vendors: { posthog: true } } },
    {
      ...valid(),
      consent: { analytics: { ...ACCEPT_ALL_CONSENT.analytics, productAnalytics: 'true' } },
    },
    {
      ...valid(),
      consent: { analytics: { ...ACCEPT_ALL_CONSENT.analytics, featureFlags: undefined } },
    },
  ])('rejects malformed or expired consent consistently: %j', (candidate) => {
    for (const read of [getTrackingConsent, readTrackingConsentStoredValue]) {
      values.set(key, JSON.stringify(candidate));
      for (const version of [1, 2, 3]) values.set(`nvbes.tracking-consent.v${version}`, 'accepted');
      values.set('session', 'preserve');
      expect(read()).toBeNull();
      expect([...values]).toEqual([['session', 'preserve']]);
      expect(getAnalyticsConsent()).toEqual(DECLINE_ALL_CONSENT.analytics);
    }
  });

  it('clears malformed JSON without throwing or granting optional consent', () => {
    values.set(key, '{');
    expect(getTrackingConsent()).toBeNull();
    expect(values.has(key)).toBe(false);
    expect(isCategoryAccepted('analytics')).toBe(false);
    expect(isVendorAccepted('posthog')).toBe(false);
    expect(isAnalyticsPurposeAccepted('sessionReplay')).toBe(false);
  });

  it('round-trips metadata and derives vendors from explicit purposes only', () => {
    persistTrackingConsent(ACCEPT_ALL_CONSENT, 'test');
    const expiry = new Date(now);
    expiry.setDate(expiry.getDate() + 183);
    expect(readTrackingConsentStoredValue()).toEqual({
      version: 4,
      savedAt: now.toISOString(),
      expiresAt: expiry.toISOString(),
      source: 'test',
      consent: ACCEPT_ALL_CONSENT,
    });
    expect(isCategoryAccepted('analytics')).toBe(true);
    expect(isVendorAccepted('posthog')).toBe(true);
    expect(isAnalyticsPurposeAccepted('productAnalytics')).toBe(true);
    values.set(
      key,
      JSON.stringify({
        ...valid(),
        consent: { ...ACCEPT_ALL_CONSENT, analytics: DECLINE_ALL_CONSENT.analytics },
      }),
    );
    expect(getTrackingConsent()).toEqual(DECLINE_ALL_CONSENT);
    expect(isCategoryAccepted('analytics')).toBe(false);
    expect(isVendorAccepted('posthog')).toBe(false);
  });

  it('expires at the boundary but accepts the preceding millisecond', () => {
    const expiry = new Date(now.getTime() + 1).toISOString();
    values.set(key, JSON.stringify({ ...valid(), expiresAt: expiry }));
    expect(getTrackingConsent()).toEqual(ACCEPT_ALL_CONSENT);
    vi.setSystemTime(new Date(expiry));
    expect(readTrackingConsentStoredValue()).toBeNull();
    expect(values.has(key)).toBe(false);
  });

  it('returns independent copies and does not mutate input state', () => {
    const copy = cloneConsent(ACCEPT_ALL_CONSENT);
    expect(copy).toEqual(ACCEPT_ALL_CONSENT);
    for (const section of ['categories', 'vendors', 'analytics'] as const) {
      expect(copy[section]).not.toBe(ACCEPT_ALL_CONSENT[section]);
    }
    persistTrackingConsent(copy, 'test');
    copy.analytics.productAnalytics = false;
    const first = getAnalyticsConsent();
    first.productAnalytics = false;
    expect(getAnalyticsConsent().productAnalytics).toBe(true);
    expect(hasAnyOptionalConsent(ACCEPT_ALL_CONSENT)).toBe(true);
    expect(hasAnyOptionalConsent(DECLINE_ALL_CONSENT)).toBe(false);
  });
});
