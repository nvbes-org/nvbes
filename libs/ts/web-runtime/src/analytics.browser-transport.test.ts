import { afterEach, describe, expect, it, vi } from 'vite-plus/test';
import { createBrowserAnalyticsTransport } from './analytics.browser-transport';

const posthog = vi.hoisted(() => ({
  capture: vi.fn(),
  getFeatureFlag: vi.fn(),
  getFeatureFlagPayload: vi.fn(),
  group: vi.fn(),
  identify: vi.fn(),
  init: vi.fn(),
  opt_in_capturing: vi.fn(),
  opt_out_capturing: vi.fn(),
  reset: vi.fn(),
  startSessionRecording: vi.fn(),
  stopSessionRecording: vi.fn(),
}));

vi.mock('posthog-js', () => ({ default: posthog }));

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
});
