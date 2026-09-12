// @vitest-environment happy-dom
import { afterEach, beforeEach, expect, it, vi } from 'vite-plus/test';
import { collectExtensionSignals } from './bot-guard.signals.extensions';

beforeEach(() => {
  const storage = new Map<string, string>();
  vi.stubGlobal('localStorage', {
    getItem: (key: string) => storage.get(key) ?? null,
    setItem: (key: string, value: string) => storage.set(key, value),
  });
  vi.stubEnv('DEV', false);
  vi.spyOn(navigator, 'userAgent', 'get').mockReturnValue('Safari');
  vi.spyOn(HTMLElement.prototype, 'offsetHeight', 'get').mockReturnValue(1);
  vi.stubGlobal('fetch', vi.fn().mockRejectedValue(new Error('absent')));
});
afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  vi.unstubAllEnvs();
  vi.useRealTimers();
  document.body.replaceChildren();
});

it('returns neutral signals without extension resource support or DOM markers', async () => {
  expect(await collectExtensionSignals()).toEqual({
    ext_blacklisted_detected: false,
    ext_blacklisted_list: [],
    ext_adblocker_active: false,
    ext_adblocker_list: [],
    ext_dom_tampered: false,
  });
  expect(fetch).not.toHaveBeenCalled();
  expect(document.body.childElementCount).toBe(0);
});

it.each(['Chrome', 'Firefox'])(
  'separates automation from allowed adblockers on %s',
  async (browser) => {
    vi.spyOn(navigator, 'userAgent', 'get').mockReturnValue(browser);
    const statuses = [200, 404, 0, 500, 200, 403, 0, 404];
    vi.mocked(fetch).mockImplementation(async (_url, options) => {
      expect(options).toMatchObject({
        method: 'HEAD',
        mode: 'no-cors',
        signal: expect.any(AbortSignal),
      });
      return { status: statuses.shift() } as Response;
    });
    expect(await collectExtensionSignals()).toMatchObject({
      ext_blacklisted_detected: true,
      ext_blacklisted_list: ['Selenium IDE', 'Web Scraper'],
      ext_adblocker_active: true,
      ext_adblocker_list: ['uBlock Origin', 'AdBlock'],
    });
    expect(fetch).toHaveBeenCalledTimes(8);
    expect(
      vi.mocked(fetch).mock.calls.every(([url]) => String(url).startsWith('chrome-extension://')),
    ).toBe(true);
  },
);

it('aborts all unavailable probes at 120ms without reporting a detection', async () => {
  vi.useFakeTimers();
  vi.spyOn(navigator, 'userAgent', 'get').mockReturnValue('Chrome');
  vi.mocked(fetch).mockImplementation(
    (_url, options) =>
      new Promise((_resolve, reject) => {
        options?.signal?.addEventListener('abort', () =>
          reject(new DOMException('aborted', 'AbortError')),
        );
      }),
  );
  const result = collectExtensionSignals();
  await vi.advanceTimersByTimeAsync(119);
  expect(vi.getTimerCount()).toBe(8);
  await vi.advanceTimersByTimeAsync(1);
  expect(await result).toMatchObject({
    ext_blacklisted_detected: false,
    ext_adblocker_active: false,
  });
  expect(vi.getTimerCount()).toBe(0);
});

it.each(['display', 'visibility', 'height'])(
  'detects DOM adblocking through %s without blacklisting',
  async (reason) => {
    if (reason === 'height')
      vi.spyOn(HTMLElement.prototype, 'offsetHeight', 'get').mockReturnValue(0);
    else
      vi.spyOn(window, 'getComputedStyle').mockReturnValue({
        display: reason === 'display' ? 'none' : 'block',
        visibility: reason === 'visibility' ? 'hidden' : 'visible',
      } as CSSStyleDeclaration);
    expect(await collectExtensionSignals()).toMatchObject({
      ext_blacklisted_detected: false,
      ext_adblocker_list: ['Generic Adblocker'],
    });
    expect(document.body.childElementCount).toBe(0);
  },
);

it('removes the advertisement decoy even when the browser rejects style inspection', async () => {
  vi.spyOn(window, 'getComputedStyle').mockImplementation(() => {
    throw new Error('denied');
  });
  expect((await collectExtensionSignals()).ext_adblocker_active).toBe(false);
  expect(document.body.childElementCount).toBe(0);
});

it.each(['data-webscraper-active', 'data-autofill-active'])(
  'detects injected marker %s',
  async (attribute) => {
    const marker = document.createElement('div');
    marker.setAttribute(attribute, '');
    document.body.append(marker);
    expect((await collectExtensionSignals()).ext_dom_tampered).toBe(true);
  },
);

it.each(['data-autofill', 'class'])('detects automation input marker %s', async (attribute) => {
  const input = document.createElement('input');
  input.setAttribute(attribute, attribute === 'class' ? 'autofilled-by-extension' : 'true');
  document.body.append(input);
  expect((await collectExtensionSignals()).ext_dom_tampered).toBe(true);
});

it('does not mistake a regular input for automation and tolerates DOM inspection errors', async () => {
  document.body.append(document.createElement('input'));
  expect((await collectExtensionSignals()).ext_dom_tampered).toBe(false);
  vi.spyOn(document, 'querySelector').mockImplementation(() => {
    throw new Error('denied');
  });
  expect((await collectExtensionSignals()).ext_dom_tampered).toBe(false);
});

it.each([false, true])('honors development overrides only with DEV=%s', async (development) => {
  vi.stubEnv('DEV', development);
  for (const key of ['blacklisted', 'adblocker', 'dom_tampered'])
    localStorage.setItem(`__bg_mock_ext_${key}`, 'true');
  const result = await collectExtensionSignals();
  expect(result.ext_blacklisted_detected).toBe(development);
  expect(result.ext_adblocker_active).toBe(development);
  expect(result.ext_dom_tampered).toBe(development);
  expect(result.ext_blacklisted_list).toEqual(
    development ? ['Selenium IDE (Mocked)', 'Web Scraper (Mocked)'] : [],
  );
  expect(result.ext_adblocker_list).toEqual(development ? ['uBlock Origin (Mocked)'] : []);
});
