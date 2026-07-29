import { describe, expect, it } from 'vite-plus/test';

import {
  detectBrowser,
  detectBrowserVersion,
  detectDevice,
  detectDeviceType,
  detectOS,
  parseUserAgent,
} from '../user-agent';

const IPHONE_SAFARI =
  'Mozilla/5.0 (iPhone; CPU iPhone OS 17_5 like Mac OS X) AppleWebKit/605.1.15 ' +
  '(KHTML, like Gecko) Version/17.5 Mobile/15E148 Safari/604.1';
const ANDROID_CHROME =
  'Mozilla/5.0 (Linux; Android 14; Pixel 8 Pro) AppleWebKit/537.36 ' +
  '(KHTML, like Gecko) Chrome/126.0.0.0 Mobile Safari/537.36';

describe('browser detection', () => {
  it('detects Safari and its major/minor version', () => {
    expect(detectBrowser(IPHONE_SAFARI, 'Apple Computer, Inc.')).toBe('Mobile Safari');
    expect(detectBrowserVersion(IPHONE_SAFARI, 'Apple Computer, Inc.')).toBe(17.5);
  });

  it('prioritizes Chromium derivatives over Chrome', () => {
    const edge =
      'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 ' +
      '(KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36 Edg/126.0.2592.87';
    expect(detectBrowser(edge)).toBe('Microsoft Edge');
    expect(detectBrowserVersion(edge)).toBe(126);
  });

  it('uses client hints for browsers hidden in the user agent', () => {
    expect(detectBrowser(ANDROID_CHROME, '', { brave: true })).toBe('Brave');
  });

  it('keeps Google Search App detection opt-in', () => {
    const googleApp = `${IPHONE_SAFARI} GSA/321.0.654321`;
    expect(detectBrowser(googleApp)).toBe('Mobile Safari');
    expect(detectBrowser(googleApp, '', undefined, { detectGoogleSearchApp: true })).toBe(
      'Google Search App',
    );
  });
});

describe('operating system and device detection', () => {
  it('extracts normalized OS versions', () => {
    expect(detectOS(IPHONE_SAFARI)).toEqual(['iOS', '17.5.0']);
    expect(detectOS(ANDROID_CHROME)).toEqual(['Android', '14.0.0']);
    expect(detectOS('Mozilla/5.0 (Windows NT 10.0; Win64; x64)')).toEqual(['Windows', '10']);
  });

  it('distinguishes Android phones and tablets', () => {
    expect(detectDevice(ANDROID_CHROME)).toBe('Android');
    expect(detectDeviceType(ANDROID_CHROME)).toBe('Mobile');
    expect(detectDeviceType('Mozilla/5.0 (Linux; Android 13; Pixel C) Safari/537.36')).toBe(
      'Tablet',
    );
  });

  it('supports consoles and user-agent client hint fallbacks', () => {
    expect(detectDeviceType('Mozilla/5.0 (PlayStation 5 3.20) AppleWebKit/605.1.15')).toBe(
      'Console',
    );
    expect(
      detectDeviceType('reduced-user-agent', {
        userAgentDataPlatform: 'Android',
        maxTouchPoints: 5,
        screenWidth: 1_600,
        screenHeight: 2_560,
        devicePixelRatio: 2,
      }),
    ).toBe('Tablet');
  });
});

describe('parseUserAgent', () => {
  it('returns one coherent summary', () => {
    expect(parseUserAgent(ANDROID_CHROME)).toEqual({
      browser: 'Chrome',
      browserVersion: 126,
      device: 'Android',
      deviceType: 'Mobile',
      os: 'Android',
      osVersion: '14.0.0',
    });
  });
});
