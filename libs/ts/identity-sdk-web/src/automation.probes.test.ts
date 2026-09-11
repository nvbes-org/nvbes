// @vitest-environment happy-dom
import { afterEach, expect, it, vi } from 'vite-plus/test';
import {
  probeWebdriver,
  probeChromeDriver,
  probeGlobalTools,
  probeChromeRuntime,
  probePlugins,
  probeNativeFunctionSpoofing,
} from './bot-guard.signals.automation.probes.basic';
import {
  probeChromeSpoofing,
  probeIframeWebdriver,
  probeUADataInconsistency,
} from './bot-guard.signals.automation.probes.advanced';

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});
it('distinguishes native webdriver from an own-property override', () => {
  vi.stubGlobal('navigator', Object.create({ webdriver: false }));
  expect(probeWebdriver()).toEqual({ webdriver: false, spoofed: false });
  vi.stubGlobal('navigator', { webdriver: true });
  expect(probeWebdriver()).toEqual({ webdriver: true, spoofed: true });
  vi.spyOn(Object, 'getOwnPropertyDescriptor').mockImplementation(() => {
    throw new Error('denied');
  });
  expect(probeWebdriver().spoofed).toBe(true);
});
it.each([
  '_selenium',
  'callSelenium',
  '_phantom',
  'callPhantom',
  'Cypress',
  '__cypress',
  '__playwright',
  '__playwright_coverage',
  '__playwright_evaluation',
  '__nightmare',
  'domAutomation',
  'domAutomationController',
])('recognizes global automation marker %s', (key) => {
  vi.stubGlobal('window', {});
  expect(probeGlobalTools()).toBe(false);
  vi.stubGlobal('window', { [key]: null });
  expect(probeGlobalTools()).toBe(true);
});
it.each(['window', 'document'])('detects injected Chrome driver on %s', (target) => {
  vi.stubGlobal('window', {});
  vi.stubGlobal('document', {});
  expect(probeChromeDriver()).toBe(false);
  vi.stubGlobal(target, { $cdc_script: true });
  expect(probeChromeDriver()).toBe(true);
  vi.stubGlobal(
    target,
    new Proxy(
      {},
      {
        ownKeys() {
          throw new Error('denied');
        },
      },
    ),
  );
  expect(probeChromeDriver()).toBe(false);
});
it.each([
  ['Chrome', {}, true],
  ['Chrome Mobile', {}, false],
  ['Firefox', {}, false],
  ['Chrome', { runtime: {} }, false],
  ['Chrome', undefined, false],
])('checks runtime for %s', (userAgent, chrome, expected) => {
  vi.stubGlobal('navigator', { userAgent });
  vi.stubGlobal('window', { chrome });
  expect(probeChromeRuntime()).toBe(expected);
});
it.each([
  ['Windows', 'Windows', false],
  ['Windows', 'Win64', false],
  ['Windows', 'WOW64', false],
  ['Windows', 'Linux', true],
  ['macOS', 'Macintosh', false],
  ['macOS', 'Mac OS', false],
  ['macOS', 'Linux', true],
  ['Linux', 'Linux', false],
  ['Linux', 'Android', true],
  ['Linux', 'Windows', true],
  ['', 'Linux', false],
])('compares platform %s to agent %s', (platform, userAgent, expected) => {
  vi.stubGlobal('navigator', { userAgent, userAgentData: { platform, mobile: false } });
  expect(probeUADataInconsistency()).toBe(expected);
});
it.each(['Android', 'iPhone', 'iPad', 'Mobile'])(
  'accepts consistent mobile agent %s',
  (userAgent) => {
    vi.stubGlobal('navigator', { userAgent, userAgentData: { platform: 'Other', mobile: true } });
    expect(probeUADataInconsistency()).toBe(false);
  },
);
it('does not infer inconsistency when UA data is unavailable', () => {
  vi.stubGlobal('navigator', {});
  expect(probeUADataInconsistency()).toBe(false);
});
it.each([
  'Firefox',
  'Chrome Mobile',
  'Chrome Android',
  'Chrome iPhone',
  'Chrome iPad',
  'Chrome Edg',
  'Chrome OPR',
  'Chrome Safari/123.4',
])('does not require desktop Chrome runtime for %s', (userAgent) => {
  vi.stubGlobal('navigator', { userAgent });
  vi.stubGlobal('window', {});
  expect(probeChromeSpoofing()).toBe(false);
});
it('recognizes absent, valid and accessor-provided Chrome runtime', () => {
  vi.stubGlobal('navigator', { userAgent: 'Chrome' });
  vi.stubGlobal('window', {});
  expect(probeChromeSpoofing()).toBe(true);
  vi.stubGlobal('window', { chrome: {} });
  expect(probeChromeSpoofing()).toBe(true);
  vi.stubGlobal('window', { chrome: { runtime: {} } });
  expect(probeChromeSpoofing()).toBe(false);
  vi.stubGlobal('window', {
    get chrome() {
      return { runtime: {} };
    },
  });
  expect(probeChromeSpoofing()).toBe(true);
});
it('recognizes plugins without flagging a native-shaped collection', () => {
  vi.stubGlobal('navigator', {});
  expect(probePlugins()).toBe(false);
  vi.stubGlobal('navigator', { plugins: [] });
  expect(probePlugins()).toBe(true);
  vi.stubGlobal('navigator', { plugins: { [Symbol.toStringTag]: 'PluginArray' } });
  expect(probePlugins()).toBe(false);
});
it('detects altered native function representations', () => {
  const native = { toString: () => 'function query() { [native code] }' };
  vi.stubGlobal('navigator', { permissions: { query: native } });
  vi.stubGlobal('HTMLCanvasElement', { prototype: { toDataURL: native } });
  expect(probeNativeFunctionSpoofing()).toBe(false);
  vi.stubGlobal('navigator', {
    permissions: { query: { toString: () => 'return [native code]' } },
  });
  expect(probeNativeFunctionSpoofing()).toBe(true);
});
it.each([true, false])('removes its iframe after checking webdriver=%s', (webdriver) => {
  const create = document.createElement.bind(document);
  vi.spyOn(document, 'createElement').mockImplementation((tag) => {
    const element = create(tag);
    if (tag === 'iframe')
      Object.defineProperty(element, 'contentWindow', { value: { navigator: { webdriver } } });
    return element;
  });
  expect(probeIframeWebdriver()).toBe(webdriver);
  expect(document.head.querySelector('iframe')).toBeNull();
});
it('contains inaccessible browser APIs', () => {
  vi.stubGlobal(
    'navigator',
    new Proxy(
      {},
      {
        get() {
          throw new Error('denied');
        },
      },
    ),
  );
  expect(probeUADataInconsistency()).toBe(false);
  expect(probeChromeSpoofing()).toBe(true);
  expect(probeChromeRuntime()).toBe(false);
  expect(probePlugins()).toBe(true);
  expect(probeNativeFunctionSpoofing()).toBe(true);
  vi.stubGlobal(
    'window',
    new Proxy(
      {},
      {
        get() {
          throw new Error('denied');
        },
      },
    ),
  );
  expect(probeGlobalTools()).toBe(false);
  vi.stubGlobal('document', undefined);
  expect(probeIframeWebdriver()).toBe(false);
});
