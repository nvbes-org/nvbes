import { describe, expect, it } from 'vite-plus/test';
import { RuntimeAnalytics } from './analytics.runtime';
import { EMPTY_ANALYTICS_CONSENT } from './analytics.types';

describe('RuntimeAnalytics consent lifecycle', () => {
  it('disables capture on init when analytics consent is empty', async () => {
    let disabled = 0;
    let replayStops = 0;

    const runtime = new RuntimeAnalytics({
      appName: 'identity-web',
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
      appName: 'identity-web',
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
});
