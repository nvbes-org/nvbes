import { expect, it } from 'vite-plus/test';
import { detectBrowser, detectBrowserVersion } from './user-agent.browser';

it.each([
  ['Chromium/124.5.6', 'Chrome', 124.5],
  ['CrMo/42.1', 'Chrome', 42.1],
  ['CriOS/125.1', 'Chrome iOS', 125.1],
  ['Firefox/120.2', 'Firefox', 120.2],
  ['FxiOS/120.3', 'Firefox iOS', 120.3],
  [' OPR/99.1', 'Opera', 99.1],
  [' OPR/99 Mini', 'Opera Mini', null],
  ['BlackBerry 10.3', 'BlackBerry', 10.3],
  ['PlayBook Version/2.1', 'BlackBerry', 2.1],
  ['BB10 Version/10.2', 'BlackBerry', 10.2],
  ['IEMobile rv:11.0', 'Internet Explorer Mobile', 11],
  ['WPDesktop rv:11.1', 'Internet Explorer Mobile', 11.1],
  ['OculusBrowser/20.2 Chrome/110', 'Oculus Browser', 20.2],
  ['SamsungBrowser/22.1 Chrome/110', 'Samsung Internet', 22.1],
  ['Edg/123.4 Chrome/110', 'Microsoft Edge', 123.4],
  ['EdgA/123.4 Chrome/110', 'Microsoft Edge', 123.4],
  ['EdgiOS/123.4', 'Microsoft Edge', 123.4],
  ['Edge/18.5', 'Microsoft Edge', 18.5],
  ['Vivaldi/6.5 Chrome/110', 'Vivaldi', 6.5],
  ['YaBrowser/23.1 Chrome/110', 'Yandex', 23.1],
  ['Whale/3.2 Chrome/110', 'Whale', 3.2],
  ['DuckDuckGo/1.2', 'DuckDuckGo', 1.2],
  ['Ddg/1.3', 'DuckDuckGo', 1.3],
  ['FBIOS', 'Facebook Mobile', null],
  ['UCWEB/12.1', 'UC Browser', 12.1],
  ['UCBrowser/12.2', 'UC Browser', 12.2],
  ['Android 13.1 Safari', 'Android Mobile', 13.1],
  ['Konqueror/4.5', 'Konqueror', 4.5],
  ['Brave/1.2', 'Brave', 1.2],
  ['Version/17.1 Mobile Safari', 'Mobile Safari', 17.1],
  ['Version/17.2 Safari', 'Safari', 17.2],
  ['PaleMoon/33.1 Gecko', 'Pale Moon', 33.1],
  ['Waterfox/6.2 Gecko', 'Waterfox', 6.2],
  ['Gecko', 'Firefox', null],
  ['MSIE 10.1', 'Internet Explorer', 10.1],
  ['Trident/7 rv:11.1', 'Internet Explorer', 11.1],
  ['unrecognized', null, null],
] as const)('classifies %s without confusing embedded browser tokens', (ua, name, version) => {
  expect(detectBrowser(ua)).toBe(name);
  expect(detectBrowserVersion(ua)).toBe(version);
});
it('uses explicit browser hints and keeps Google Search detection opt-in', () => {
  expect(detectBrowser('Chrome/120', '', { brave: true })).toBe('Brave');
  expect(detectBrowser('GSA/12.3 Safari')).toBe('Safari');
  expect(
    detectBrowserVersion('GSA/12.3 Safari', '', undefined, { detectGoogleSearchApp: true }),
  ).toBe(12.3);
  expect(detectBrowser('Mobile', 'Apple')).toBe('Mobile Safari');
  expect(detectBrowser('', 'Apple')).toBe('Safari');
  expect(detectBrowserVersion('Chrome/not-a-number')).toBeNull();
});
