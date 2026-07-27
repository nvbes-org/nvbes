import { describe, expect, it } from 'vite-plus/test';
import { RuntimeAnalytics } from './analytics.runtime';
import { EMPTY_ANALYTICS_CONSENT } from './analytics.types';

describe('RuntimeAnalytics consent lifecycle', () => {
  it('applies browser error reporting consent independently', async () => {
    const states: boolean[] = [];
    const runtime = new RuntimeAnalytics({
      appName: 'account-web',
      getConsent: () => ({
        ...EMPTY_ANALYTICS_CONSENT,
        productAnalytics: true,
        errorTracking: true,
      }),
      transport: {
        setErrorReportingEnabled: (enabled) => {
          states.push(enabled);
        },
      },
    });

    await runtime.init();
    await runtime.applyConsent({
      ...EMPTY_ANALYTICS_CONSENT,
      productAnalytics: true,
    });

    expect(states).toEqual([true, false]);
  });

  it('keeps the PostHog transport active when error tracking remains', async () => {
    const states: boolean[] = [];
    const runtime = new RuntimeAnalytics({
      appName: 'account-web',
      getConsent: () => ({
        ...EMPTY_ANALYTICS_CONSENT,
        productAnalytics: true,
      }),
      transport: {
        setProductAnalyticsEnabled: (enabled) => {
          states.push(enabled);
        },
      },
    });

    await runtime.init();
    await runtime.applyConsent({
      ...EMPTY_ANALYTICS_CONSENT,
      errorTracking: true,
    });

    expect(states).toEqual([true, true]);
  });

  it('disables capture on init when analytics consent is empty', async () => {
    let disabled = 0;
    let replayStops = 0;

    const runtime = new RuntimeAnalytics({
      appName: 'account-web',
      getConsent: () => EMPTY_ANALYTICS_CONSENT,
      transport: {
        disableCapture: () => {
          disabled += 1;
        },
        stopPrivacySafeReplay: () => {
          replayStops += 1;
        },
      },
    });

    await runtime.init();

    expect(disabled).toBe(1);
    expect(replayStops).toBe(0);
  });

  it('stops replay as a fallback when transport has no disable hook', async () => {
    let replayStops = 0;

    const runtime = new RuntimeAnalytics({
      appName: 'account-web',
      getConsent: () => EMPTY_ANALYTICS_CONSENT,
      transport: {
        stopPrivacySafeReplay: () => {
          replayStops += 1;
        },
      },
    });

    await runtime.init();

    expect(replayStops).toBe(1);
  });

  it('exposes safe correlation identifiers only with product analytics consent', async () => {
    const runtime = new RuntimeAnalytics({
      appName: 'account-web',
      getConsent: () => ({
        ...EMPTY_ANALYTICS_CONSENT,
        productAnalytics: true,
      }),
      transport: {
        getCorrelationContext: () => ({
          distinctId: 'distinct_12345678',
          sessionId: 'session_12345678',
        }),
      },
    });

    await expect(runtime.getCorrelationContext()).resolves.toEqual({
      distinctId: 'distinct_12345678',
      sessionId: 'session_12345678',
    });
  });
});
