import { afterEach, expect, it, vi } from 'vite-plus/test';
import { RuntimeAnalytics } from './analytics.runtime';
import {
  ALL_ANALYTICS_CONSENT,
  EMPTY_ANALYTICS_CONSENT,
  type AnalyticsPurposeConsent,
} from './analytics.types';

afterEach(() => vi.unstubAllGlobals());
function setup(consent = ALL_ANALYTICS_CONSENT, route = '/dashboard') {
  const transport = {
    trackProductEvent: vi.fn(),
    identifyProductUser: vi.fn(),
    setWorkspaceGroup: vi.fn(),
    getFeatureFlag: vi.fn().mockReturnValue(true),
    getFeatureFlagPayload: vi.fn().mockReturnValue({ variant: 'a' }),
    getCorrelationContext: vi
      .fn()
      .mockReturnValue({ distinctId: 'distinct_123', sessionId: 'session_123' }),
    captureException: vi.fn(),
    startPrivacySafeReplay: vi.fn(),
    stopPrivacySafeReplay: vi.fn(),
    setProductAnalyticsEnabled: vi.fn(),
    setErrorReportingEnabled: vi.fn(),
    disableCapture: vi.fn(),
  };
  const runtime = new RuntimeAnalytics({
    appName: 'test',
    getConsent: () => consent,
    getRoutePath: () => route,
    transport,
  });
  return { runtime, transport };
}
it('forwards only approved events and sanitized properties', async () => {
  const { runtime, transport } = setup();
  await runtime.trackProductEvent('unknown');
  expect(transport.trackProductEvent).not.toHaveBeenCalled();
  await runtime.trackProductEvent('marketing.page_viewed', {
    plan_code: 'free',
    email: 'private@example.test',
    duration_ms: 20,
    token: 'secret',
  });
  expect(transport.trackProductEvent).toHaveBeenCalledExactlyOnceWith('marketing.page_viewed', {
    route_path: '/dashboard',
    plan_code: 'free',
    duration_ms: 20,
  });
});
it.each([EMPTY_ANALYTICS_CONSENT, ALL_ANALYTICS_CONSENT])(
  'gates all sensitive-route capture paths (%j)',
  async (consent) => {
    const { runtime, transport } = setup(consent, '/account/security');
    await runtime.trackProductEvent('$pageview');
    await runtime.captureAnalyticsException(new Error('private'));
    await runtime.startPrivacySafeReplay();
    await expect(runtime.getFeatureFlag('flag')).resolves.toBeUndefined();
    await expect(runtime.getFeatureFlagPayload('flag')).resolves.toBeUndefined();
    await expect(runtime.isFeatureEnabled('flag')).resolves.toBe(false);
    await expect(runtime.getCorrelationContext()).resolves.toBeUndefined();
    for (const fn of [
      transport.trackProductEvent,
      transport.captureException,
      transport.startPrivacySafeReplay,
      transport.getFeatureFlag,
      transport.getFeatureFlagPayload,
      transport.getCorrelationContext,
    ])
      expect(fn).not.toHaveBeenCalled();
  },
);
it('returns flags, payloads and explicit boolean enablement', async () => {
  const { runtime, transport } = setup();
  await expect(runtime.getFeatureFlag('flag')).resolves.toBe(true);
  await expect(runtime.getFeatureFlagPayload('flag')).resolves.toEqual({ variant: 'a' });
  for (const value of [true, false, 'true', 1, null, undefined]) {
    transport.getFeatureFlag.mockReturnValue(value);
    await expect(runtime.isFeatureEnabled('flag')).resolves.toBe(value === true);
  }
  await runtime.trackExperimentExposure('flag', 'a');
  expect(transport.trackProductEvent).toHaveBeenCalledWith('analytics.experiment_exposure', {
    feature_flag: 'flag',
    variant: 'a',
    route_path: '/dashboard',
  });
  await runtime.applyConsent(EMPTY_ANALYTICS_CONSENT);
  transport.trackProductEvent.mockClear();
  await runtime.trackExperimentExposure('flag', 'b');
  expect(transport.trackProductEvent).not.toHaveBeenCalled();
});
it.each(['short', 'a'.repeat(201), 'person@example.test', 'with space', 'a/b'])(
  'rejects unsafe correlation identifier %s',
  async (unsafe) => {
    const { runtime, transport } = setup();
    for (const context of [
      { distinctId: unsafe, sessionId: 'session_123' },
      { distinctId: 'distinct_123', sessionId: unsafe },
      undefined,
    ]) {
      transport.getCorrelationContext.mockReturnValue(context);
      await expect(runtime.getCorrelationContext()).resolves.toBeUndefined();
    }
  },
);
it('pseudonymizes account and workspace IDs and rejects identifiers that are not UUIDs', async () => {
  const { runtime, transport } = setup();
  const id = '12345678-1234-1234-8123-123456789abc';
  await runtime.identifyProductUser('invalid');
  await runtime.setAnalyticsWorkspaceGroup('invalid');
  expect(transport.identifyProductUser).not.toHaveBeenCalled();
  expect(transport.setWorkspaceGroup).not.toHaveBeenCalled();
  await runtime.identifyProductUser(id, { role: 'owner', email: 'private@example.test' });
  await runtime.setAnalyticsWorkspaceGroup(id, { plan_code: 'free' });
  expect(transport.identifyProductUser).toHaveBeenCalledWith(
    expect.stringMatching(/^usr_[a-f0-9]{32}$/),
    { role: 'owner' },
  );
  expect(transport.setWorkspaceGroup).toHaveBeenCalledWith(
    expect.stringMatching(/^wks_[a-f0-9]{32}$/),
    { plan_code: 'free' },
  );
  await runtime.applyConsent(EMPTY_ANALYTICS_CONSENT);
  await runtime.identifyProductUser(id);
  await runtime.setAnalyticsWorkspaceGroup(id);
  expect(transport.identifyProductUser).toHaveBeenCalledTimes(1);
  expect(transport.setWorkspaceGroup).toHaveBeenCalledTimes(1);
});
it('uses an explicit unsupported marker without crypto support', async () => {
  vi.stubGlobal('crypto', undefined);
  const { runtime, transport } = setup();
  await runtime.identifyProductUser('12345678-1234-1234-8123-123456789abc');
  expect(transport.identifyProductUser).toHaveBeenCalledExactlyOnceWith('usr_unsupported', {});
});
it('captures exceptions only with consent and starts and stops permitted replay', async () => {
  const { runtime, transport } = setup();
  const error = new Error('private@example.test');
  await runtime.captureAnalyticsException(error, { status: 'failed' });
  expect(transport.captureException).toHaveBeenCalledWith(error, {
    status: 'failed',
    error_message: 'Redacted error message',
    route_path: '/dashboard',
  });
  await runtime.startPrivacySafeReplay();
  await runtime.stopPrivacySafeReplay();
  expect(transport.startPrivacySafeReplay).toHaveBeenCalledTimes(1);
  expect(transport.stopPrivacySafeReplay).toHaveBeenCalledTimes(1);
  await runtime.applyConsent({ ...ALL_ANALYTICS_CONSENT, sessionReplay: false });
  expect(transport.stopPrivacySafeReplay).toHaveBeenCalledTimes(2);
});
it('subscribes once and applies consent changes without duplicating subscriptions', async () => {
  let listener: ((consent: AnalyticsPurposeConsent) => void) | undefined;
  const onConsentChange = vi.fn((callback: (consent: AnalyticsPurposeConsent) => void) => {
    listener = callback;
    return vi.fn();
  });
  const { transport } = setup();
  const runtime = new RuntimeAnalytics({
    appName: 'test',
    getConsent: () => ALL_ANALYTICS_CONSENT,
    onConsentChange,
    transport,
  });
  await runtime.init();
  await runtime.init();
  expect(onConsentChange).toHaveBeenCalledTimes(1);
  listener?.(EMPTY_ANALYTICS_CONSENT);
  await vi.waitFor(() => expect(transport.disableCapture).toHaveBeenCalledTimes(1));
  await runtime.applyConsent();
  expect(transport.setProductAnalyticsEnabled).toHaveBeenLastCalledWith(true);
});
