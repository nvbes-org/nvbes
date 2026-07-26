import { afterEach, describe, expect, it, vi } from 'vite-plus/test';

import { collectBotIntegritySignals } from '../bot-integrity';

afterEach(() => {
  vi.unstubAllGlobals();
});

describe('UA-CH bot integrity signals', () => {
  it('collects low and high entropy values when the browser exposes them', async () => {
    vi.stubGlobal('navigator', {
      webdriver: false,
      maxTouchPoints: 5,
      hardwareConcurrency: 8,
      language: 'fr-FR',
      userAgentData: {
        brands: [{ brand: 'Chromium', version: '126' }],
        mobile: true,
        platform: 'Android',
        getHighEntropyValues: async () => ({
          architecture: 'arm',
          bitness: '64',
          formFactors: ['Mobile'],
          fullVersionList: [{ brand: 'Chromium', version: '126.0.1.2' }],
          model: 'Pixel 8',
          platformVersion: '14.0.0',
          wow64: false,
        }),
      },
    });

    const signals = await collectBotIntegritySignals();

    expect(signals).toMatchObject({
      ua_brands: ['Chromium/126'],
      ua_architecture: 'arm',
      ua_bitness: '64',
      ua_form_factors: ['Mobile'],
      ua_full_version_list: ['Chromium/126.0.1.2'],
      ua_model: 'Pixel 8',
      ua_mobile: true,
      ua_platform: 'Android',
      ua_platform_version: '14.0.0',
      ua_wow64: false,
    });
  });

  it('keeps authentication usable when high entropy collection is denied', async () => {
    vi.stubGlobal('navigator', {
      webdriver: false,
      maxTouchPoints: 0,
      hardwareConcurrency: 4,
      language: 'en-US',
      userAgentData: {
        brands: [{ brand: 'Chromium', version: '126' }],
        mobile: false,
        platform: 'Linux',
        getHighEntropyValues: async () => Promise.reject(new Error('privacy budget denied')),
      },
    });

    await expect(collectBotIntegritySignals()).resolves.toMatchObject({
      ua_brands: ['Chromium/126'],
      ua_mobile: false,
      ua_platform: 'Linux',
    });
  });
});
