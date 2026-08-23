import type { HeaderMap, JsonObject } from './bot-defense.support';

export interface ClientProbeAttack {
  name: string;
  initScript: string;
  expectedSignals: JsonObject;
}

export interface RequestEvasionAttack {
  name: string;
  mutateBody: (body: JsonObject) => JsonObject;
  mutateHeaders?: (headers: HeaderMap) => HeaderMap;
}

export const CLIENT_PROBE_ATTACKS: readonly ClientProbeAttack[] = [
  {
    name: 'navigator.webdriver shadowed on the navigator instance',
    initScript: `Object.defineProperty(navigator, 'webdriver', { configurable: true, get: () => false });`,
    expectedSignals: { auto_webdriver_spoofed: true },
  },
  {
    name: 'webdriver hidden only in the main realm',
    initScript: `if (window.top === window) Object.defineProperty(Navigator.prototype, 'webdriver', { configurable: true, get: () => false });`,
    expectedSignals: { auto_iframe_webdriver: true },
  },
  {
    name: 'ChromeDriver CDP globals left behind',
    initScript: `Object.defineProperty(window, 'cdc_adoQpoasnfa76pfcZLmcfl_Array', { value: [] });`,
    expectedSignals: { auto_chrome_driver_injected: true },
  },
  {
    name: 'Playwright and Selenium globals left behind',
    initScript: `Object.assign(window, { __playwright: {}, _selenium: {} });`,
    expectedSignals: { auto_global_tools_detected: true },
  },
  {
    name: 'permissions API monkey patched by a stealth plugin',
    initScript: `navigator.permissions.query = function query() { return Promise.resolve({ state: 'prompt' }); };`,
    expectedSignals: { auto_native_function_spoofed: true },
  },
  {
    name: 'navigator.plugins replaced by a plain array',
    initScript: `Object.defineProperty(navigator, 'plugins', { configurable: true, value: [] });`,
    expectedSignals: { auto_plugins_inconsistent: true },
  },
  {
    name: 'window.chrome exposed through a synthetic getter',
    initScript: `Object.defineProperty(window, 'chrome', { configurable: true, get: () => ({ runtime: {} }) });`,
    expectedSignals: { auto_chrome_spoofed: true },
  },
  {
    name: 'UA Client Hints contradict the legacy user agent',
    initScript: `Object.defineProperty(navigator, 'userAgentData', { configurable: true, value: { brands: [{ brand: 'Chromium', version: '140' }], mobile: true, platform: 'Windows' } });`,
    expectedSignals: { auto_ua_data_inconsistent: true },
  },
  {
    name: 'automation extension mutates the login DOM',
    initScript: `document.addEventListener('DOMContentLoaded', () => { const marker = document.createElement('div'); marker.dataset.webscraperActive = 'true'; document.body.append(marker); }, { once: true });`,
    expectedSignals: { ext_dom_tampered: true },
  },
] as const;

export const REQUEST_EVASION_ATTACKS: readonly RequestEvasionAttack[] = [
  {
    name: 'all client attestation removed',
    mutateBody: (body) => withoutClientAttestation(body),
  },
  {
    name: 'client attestation replaced with null',
    mutateBody: (body) => ({
      ...body,
      bot_guard: null,
      bot_signals: null,
      device_fingerprint: null,
      decoy_link_clicked: false,
    }),
  },
  {
    name: 'minimal all-clear automation report',
    mutateBody: (body) => ({
      ...body,
      bot_signals: { auto_webdriver: false },
    }),
  },
  {
    name: 'reverse-engineered human-looking telemetry',
    mutateBody: (body) => ({
      ...body,
      bot_signals: plausibleHumanSignals(body.bot_signals),
    }),
  },
  {
    name: 'browser metadata and fetch context stripped',
    mutateBody: (body) => withoutClientAttestation(body),
    mutateHeaders: (headers) =>
      withoutHeaders(headers, [
        'accept-language',
        'origin',
        'sec-ch-ua',
        'sec-ch-ua-mobile',
        'sec-ch-ua-platform',
        'sec-fetch-dest',
        'sec-fetch-mode',
        'sec-fetch-site',
      ]),
  },
  {
    name: 'legacy UA forged while Client Hints remain contradictory',
    mutateBody: (body) => ({
      ...body,
      bot_signals: plausibleHumanSignals(body.bot_signals),
      device_fingerprint: { platform: 'windows', form_factor: 'desktop' },
    }),
    mutateHeaders: (headers) => ({
      ...headers,
      'user-agent':
        'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/140.0.0.0 Safari/537.36',
    }),
  },
] as const;

export const FULL_STEALTH_INIT_SCRIPT = String.raw`
(() => {
  const nativeToString = Function.prototype.toString;
  const disguised = new WeakSet();
  const markNative = (fn) => { disguised.add(fn); return fn; };
  const stealthToString = markNative(function toString() {
    if (disguised.has(this)) return 'function ' + (this.name || '') + '() { [native code] }';
    return nativeToString.call(this);
  });
  Object.defineProperty(Function.prototype, 'toString', { configurable: true, value: stealthToString });

  const webdriverGetter = markNative(function get webdriver() { return false; });
  Object.defineProperty(Navigator.prototype, 'webdriver', { configurable: true, get: webdriverGetter });
  Object.defineProperties(Navigator.prototype, {
    hardwareConcurrency: { configurable: true, get: markNative(function get hardwareConcurrency() { return 8; }) },
    deviceMemory: { configurable: true, get: markNative(function get deviceMemory() { return 8; }) },
    languages: { configurable: true, get: markNative(function get languages() { return ['fr-FR', 'fr']; }) },
  });

  const permissionsQuery = markNative(function query(parameters) {
    return Promise.resolve({ state: parameters?.name === 'notifications' ? 'prompt' : 'denied' });
  });
  Object.defineProperty(navigator.permissions, 'query', { configurable: true, value: permissionsQuery });
  Object.defineProperty(window, 'chrome', { configurable: true, value: { app: {}, runtime: {} } });

  for (const marker of [
    '__playwright', '__playwright_coverage', '__playwright_evaluation',
    '_selenium', 'callSelenium', 'Cypress', '__cypress', 'domAutomation',
    'domAutomationController', '_phantom', 'callPhantom', '__nightmare'
  ]) delete window[marker];
  for (const key of Object.getOwnPropertyNames(window)) if (key.includes('cdc_')) delete window[key];
  for (const key of Object.getOwnPropertyNames(document)) if (key.includes('cdc_')) delete document[key];

  if (window.speechSynthesis) {
    const getVoices = markNative(function getVoices() { return [{ name: 'Thomas', lang: 'fr-FR' }]; });
    Object.defineProperty(window.speechSynthesis, 'getVoices', { configurable: true, value: getVoices });
  }
  if (document.fonts?.check) {
    const fontCheck = markNative(function check() { return true; });
    Object.defineProperty(document.fonts, 'check', { configurable: true, value: fontCheck });
  }

  const originalGetParameter = WebGLRenderingContext.prototype.getParameter;
  const getParameter = markNative(function getParameter(parameter) {
    if (parameter === 37445) return 'Intel Inc.';
    if (parameter === 37446) return 'ANGLE (Intel, Intel Iris OpenGL Engine)';
    return originalGetParameter.call(this, parameter);
  });
  Object.defineProperty(WebGLRenderingContext.prototype, 'getParameter', { configurable: true, value: getParameter });
})();
`;

function withoutClientAttestation(body: JsonObject): JsonObject {
  const tampered = { ...body };
  Reflect.deleteProperty(tampered, 'bot_guard');
  Reflect.deleteProperty(tampered, 'bot_signals');
  Reflect.deleteProperty(tampered, 'device_fingerprint');
  Reflect.deleteProperty(tampered, 'decoy_link_clicked');
  return tampered;
}

function plausibleHumanSignals(original: unknown): JsonObject {
  const signals =
    typeof original === 'object' && original !== null && !Array.isArray(original)
      ? (original as JsonObject)
      : {};
  return {
    ...signals,
    mouse_event_count: 47,
    mouse_variance: 138.4,
    mouse_path_length: 684.2,
    kb_dwell_variance: 83.7,
    kb_dwell_mean: 112.5,
    kb_flight_variance: 146.2,
    kb_flight_mean: 91.3,
    kb_sample_count: 18,
    event_cascade: 31,
    event_order_valid: true,
    keyboard_submit: false,
    email_had_focus: true,
    email_focus_before_value: true,
    caret_at_end: true,
    scroll_event_count: 2,
    scroll_speed_variance: 37.1,
    submit_visible: true,
    speech_voices_count: 12,
    nav_hw_concurrency: 8,
    nav_device_memory: 8,
    font_count: 18,
    webgl_vendor: 'Intel Inc.',
    webgl_renderer: 'ANGLE (Intel, Intel Iris OpenGL Engine)',
    auto_webdriver: false,
    auto_webdriver_spoofed: false,
    auto_chrome_driver_injected: false,
    auto_global_tools_detected: false,
    auto_chrome_runtime_missing: false,
    auto_native_function_spoofed: false,
    auto_plugins_inconsistent: false,
    auto_iframe_webdriver: false,
    auto_ua_data_inconsistent: false,
    auto_chrome_spoofed: false,
    ext_blacklisted_detected: false,
    ext_blacklisted_list: [],
    ext_dom_tampered: false,
  };
}

function withoutHeaders(headers: HeaderMap, names: readonly string[]): HeaderMap {
  const stripped = { ...headers };
  for (const name of names) Reflect.deleteProperty(stripped, name);
  return stripped;
}
