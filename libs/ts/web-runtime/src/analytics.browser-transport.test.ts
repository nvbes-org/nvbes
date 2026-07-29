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
