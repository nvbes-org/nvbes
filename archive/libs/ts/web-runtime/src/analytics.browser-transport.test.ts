import { afterEach, describe, expect, it, vi } from 'vite-plus/test';
import { createBrowserAnalyticsTransport } from './analytics.browser-transport';

const posthog = vi.hoisted(() => ({
  capture: vi.fn(),
  captureException: vi.fn(),
  get_distinct_id: vi.fn(() => 'distinct_12345678'),
  getFeatureFlag: vi.fn(),
  getFeatureFlagPayload: vi.fn(),
  get_session_id: vi.fn(() => 'session_12345678'),
  group: vi.fn(),
  identify: vi.fn(),
  init: vi.fn(),
  opt_in_capturing: vi.fn(),
  opt_out_capturing: vi.fn(),
  reset: vi.fn(),
  startSessionRecording: vi.fn(),
  stopSessionRecording: vi.fn(),
}));

vi.mock('posthog-js/dist/module.full.no-external', () => ({ default: posthog }));

afterEach(() => {
  vi.clearAllMocks();
  vi.unstubAllGlobals();
});

describe('createBrowserAnalyticsTransport', () => {
  it('never resumes capture after consent withdrawal through a transport operation', async () => {
    vi.stubGlobal('window', {});
    const transport = createBrowserAnalyticsTransport({
      appName: 'test',
      environment: 'test',
      posthogKey: 'ph_test',
    });
    await transport.setProductAnalyticsEnabled?.(true);
    await transport.setProductAnalyticsEnabled?.(false);
    expect(posthog.stopSessionRecording).toHaveBeenCalledTimes(1);
    expect(posthog.opt_out_capturing).toHaveBeenCalledTimes(1);
    expect(posthog.reset).toHaveBeenCalledExactlyOnceWith(true);
    await transport.trackProductEvent?.('marketing.page_viewed', {});
    await transport.identifyProductUser?.('user', {});
    await transport.setWorkspaceGroup?.('workspace', {});
    await transport.captureException?.(new Error('private'), {});
    await transport.startPrivacySafeReplay?.();
    await expect(transport.getFeatureFlag?.('flag')).resolves.toBeUndefined();
    await expect(transport.getFeatureFlagPayload?.('flag')).resolves.toBeUndefined();
    await expect(transport.getCorrelationContext?.()).resolves.toBeUndefined();
    for (const method of [
      posthog.capture,
      posthog.identify,
      posthog.group,
      posthog.captureException,
      posthog.startSessionRecording,
      posthog.opt_in_capturing,
    ])
      expect(method).not.toHaveBeenCalled();
    await transport.setProductAnalyticsEnabled?.(true);
    expect(posthog.opt_in_capturing).toHaveBeenCalledTimes(1);
    expect(posthog.init).toHaveBeenCalledTimes(1);
    await transport.trackProductEvent?.('marketing.page_viewed', { page: 'home' });
    expect(posthog.capture).toHaveBeenCalledExactlyOnceWith('marketing.page_viewed', {
      page: 'home',
    });
  });

  it('forwards identification, groups and replay controls only with consent', async () => {
    vi.stubGlobal('window', {});
    const transport = createBrowserAnalyticsTransport({
      appName: 'test',
      environment: 'test',
      posthogKey: ' ph_test ',
      posthogHost: ' https://analytics.example.test ',
    });
    await transport.stopPrivacySafeReplay?.();
    expect(posthog.stopSessionRecording).not.toHaveBeenCalled();
    await transport.setProductAnalyticsEnabled?.(true);
    expect(posthog.init).toHaveBeenCalledWith(
      'ph_test',
      expect.objectContaining({
        api_host: 'https://analytics.example.test',
        autocapture: false,
        disable_session_recording: true,
        person_profiles: 'identified_only',
      }),
    );
    await transport.identifyProductUser?.('user-1', { plan: 'free' });
    await transport.setWorkspaceGroup?.('workspace-1', { plan: 'free' });
    expect(posthog.identify).toHaveBeenCalledExactlyOnceWith('user-1', { plan: 'free' });
    expect(posthog.group).toHaveBeenCalledExactlyOnceWith('workspace', 'workspace-1', {
      plan: 'free',
    });
    await transport.startPrivacySafeReplay?.();
    await transport.stopPrivacySafeReplay?.();
    expect(posthog.startSessionRecording).toHaveBeenCalledTimes(1);
    expect(posthog.stopSessionRecording).toHaveBeenCalledTimes(1);
    posthog.get_session_id.mockReturnValueOnce('');
    await expect(transport.getCorrelationContext?.()).resolves.toBeUndefined();
    posthog.get_distinct_id.mockReturnValueOnce('');
    await expect(transport.getCorrelationContext?.()).resolves.toBeUndefined();
  });

  it.each([true, false, 'variant', 42, null, {}, undefined])(
    'normalizes feature flag %j',
    async (value) => {
      vi.stubGlobal('window', {});
      const transport = createBrowserAnalyticsTransport({
        appName: 'test',
        environment: 'test',
        posthogKey: 'ph_test',
      });
      await transport.setProductAnalyticsEnabled?.(true);
      posthog.getFeatureFlag.mockReturnValue(value);
      await expect(transport.getFeatureFlag?.('flag')).resolves.toEqual(
        value !== null && typeof value === 'object' ? undefined : value,
      );
      expect(posthog.getFeatureFlag).toHaveBeenCalledExactlyOnceWith('flag');
    },
  );

  it.each([
    null,
    true,
    42,
    'value',
    [1, null, 'a'],
    { nested: { ok: true } },
    [undefined],
    { invalid: undefined },
    undefined,
  ])('normalizes JSON payload %j', async (value) => {
    vi.stubGlobal('window', {});
    const transport = createBrowserAnalyticsTransport({
      appName: 'test',
      environment: 'test',
      posthogKey: 'ph_test',
    });
    await transport.setProductAnalyticsEnabled?.(true);
    posthog.getFeatureFlagPayload.mockReturnValue(value);
    const invalid =
      (Array.isArray(value) && value.some((item) => item === undefined)) ||
      (value && typeof value === 'object' && 'invalid' in value);
    await expect(transport.getFeatureFlagPayload?.('flag')).resolves.toEqual(
      invalid ? undefined : value,
    );
    expect(posthog.getFeatureFlagPayload).toHaveBeenCalledExactlyOnceWith('flag');
  });

  it('does not initialize after consent is withdrawn while the SDK import is pending', async () => {
    vi.stubGlobal('window', {});
    const transport = createBrowserAnalyticsTransport({
      appName: 'test',
      environment: 'test',
      posthogKey: 'ph_test',
    });
    const enabling = transport.setProductAnalyticsEnabled?.(true);
    await transport.setProductAnalyticsEnabled?.(false);
    await enabling;
    expect(posthog.init).not.toHaveBeenCalled();
    await transport.setProductAnalyticsEnabled?.(true);
    expect(posthog.init).toHaveBeenCalledTimes(1);
  });

  it('loads and initializes PostHog only after product analytics consent', async () => {
    vi.stubGlobal('window', {});
    const transport = createBrowserAnalyticsTransport({
      appName: 'account-web',
      environment: 'test',
      posthogKey: 'ph_test',
    });

    expect(posthog.init).not.toHaveBeenCalled();

    await transport.setProductAnalyticsEnabled?.(false);
    expect(posthog.init).not.toHaveBeenCalled();

    await Promise.all([
      transport.setProductAnalyticsEnabled?.(true),
      transport.trackProductEvent?.('marketing.page_viewed', {}),
    ]);

    expect(posthog.init).toHaveBeenCalledOnce();
    expect(posthog.init).toHaveBeenCalledWith(
      'ph_test',
      expect.objectContaining({
        before_send: expect.any(Function),
        capture_pageleave: true,
        capture_pageview: false,
        custom_personal_data_properties: ['token'],
        disable_capture_url_hashes: true,
        mask_personal_data_properties: true,
      }),
    );
    expect(posthog.capture).toHaveBeenCalledWith('marketing.page_viewed', {});
  });

  it('opts out without loading PostHog when consent was never granted', async () => {
    vi.stubGlobal('window', {});
    const transport = createBrowserAnalyticsTransport({
      appName: 'account-web',
      environment: 'test',
      posthogKey: 'ph_test',
    });

    await transport.disableCapture?.();

    expect(posthog.init).not.toHaveBeenCalled();
    expect(posthog.opt_out_capturing).not.toHaveBeenCalled();
  });

  it('exposes the PostHog correlation context only after initialization', async () => {
    vi.stubGlobal('window', {});
    const transport = createBrowserAnalyticsTransport({
      appName: 'account-web',
      environment: 'test',
      posthogKey: 'ph_test',
    });

    await transport.setProductAnalyticsEnabled?.(true);

    await expect(transport.getCorrelationContext?.()).resolves.toEqual({
      distinctId: 'distinct_12345678',
      sessionId: 'session_12345678',
    });
  });

  it('reports captured exceptions to PostHog when product analytics is active', async () => {
    vi.stubGlobal('window', {});
    const transport = createBrowserAnalyticsTransport({
      appName: 'account-web',
      environment: 'test',
      posthogKey: 'ph_test',
    });

    await transport.setProductAnalyticsEnabled?.(true);
    await transport.captureException?.(new Error('boom'), { error_name: 'Error' });

    expect(posthog.captureException).toHaveBeenCalledOnce();
  });
});
