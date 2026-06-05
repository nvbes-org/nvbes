/**
 * Bot Guard — Known bot and automation detector
 *
 * Detects indicators of known test runners, headless environments,
 * and browser automation libraries (Selenium, Puppeteer, Playwright, Cypress, Nightmare, etc.).
 *
 * Specifically targets bypass/stealth attempts (e.g. proxy/prototype spoofing).
 */

export interface AutomationSignals {
  // ---- Native Automation ----
  /** navigator.webdriver value (true/false) */
  auto_webdriver: boolean;
  /** True if navigator.webdriver is spoofed (defined on object rather than prototype) */
  auto_webdriver_spoofed: boolean;
  /** True if ChromeDriver internal signature variables are found (e.g. cdc_...) */
  auto_chrome_driver_injected: boolean;

  // ---- Injected Global Objects ----
  /** True if global objects from known test tools exist (Selenium, PhantomJS, Cypress, etc.) */
  auto_global_tools_detected: boolean;

  // ---- Headless/Stealth Inconsistencies ----
  /** True if window.chrome is present but chrome.runtime is missing (headless artifact) */
  auto_chrome_runtime_missing: boolean;
  /** True if common native functions (e.g. permissions.query) are spoofed (non-native toString) */
  auto_native_function_spoofed: boolean;
  /** True if navigator.plugins is empty on a desktop UA or is an array with custom properties */
  auto_plugins_inconsistent: boolean;

  // ---- Advanced Protections ----
  /** True if a dynamically spawned clean iframe leaks navigator.webdriver === true */
  auto_iframe_webdriver: boolean;
  /** True if navigator.userAgentData and User-Agent headers contradict each other */
  auto_ua_data_inconsistent: boolean;
  /** True if window.chrome is absent or spoofed with non-native descriptors on Chrome UA */
  auto_chrome_spoofed: boolean;
}

// ---------------------------------------------------------------------------
// Probes
// ---------------------------------------------------------------------------

/** Checks navigator.webdriver and if it's been spoofed */
function probeWebdriver(): { webdriver: boolean; spoofed: boolean } {
  const hasWebdriver = !!navigator.webdriver;
  let spoofed = false;

  try {
    // In modern browsers, webdriver is a getter on Navigator.prototype.
    // If it's configured directly on the navigator instance, it's a spoof!
    const descriptor = Object.getOwnPropertyDescriptor(navigator, 'webdriver');
    if (descriptor !== undefined) {
      spoofed = true;
    }

    // In Chrome, if navigator.webdriver is redefined, sometimes the prototype descriptor is missing or modified.
    const protoDescriptor = Object.getOwnPropertyDescriptor(Navigator.prototype, 'webdriver');
    if (protoDescriptor && typeof protoDescriptor.get !== 'function') {
      spoofed = true;
    }
  } catch {
    // If an error is thrown during inspection, it's a sign of defensive proxy shielding.
    spoofed = true;
  }

  return { webdriver: hasWebdriver, spoofed };
}

/** Detects ChromeDriver's cdc_ variables inside Document or Window */
function probeChromeDriver(): boolean {
  try {
    // ChromeDriver injects unique variables starting with '$cdc_' or containing 'cdc_' on document or window.
    const keys = Object.getOwnPropertyNames(window);
    for (const key of keys) {
      if (key.includes('cdc_') || key.startsWith('$cdc_')) {
        return true;
      }
    }

    const docKeys = Object.getOwnPropertyNames(document);
    for (const key of docKeys) {
      if (key.includes('cdc_') || key.startsWith('$cdc_')) {
        return true;
      }
    }
  } catch {
    // no-op
  }
  return false;
}

/** Detects injected globals from Selenium, Cypress, Playwright, PhantomJS, Nightmare */
function probeGlobalTools(): boolean {
  try {
    const win = window as unknown as Record<string, unknown>;

    // Selenium & PhantomJS
    if (
      win._selenium !== undefined ||
      win.callSelenium !== undefined ||
      win._phantom !== undefined ||
      win.callPhantom !== undefined
    ) {
      return true;
    }

    // Cypress
    if (win.Cypress !== undefined || win.__cypress !== undefined) {
      return true;
    }

    // Playwright
    if (
      win.__playwright !== undefined ||
      win.__playwright_coverage !== undefined ||
      win.__playwright_evaluation !== undefined
    ) {
      return true;
    }

    // Nightmare & WebdriverIO
    if (
      win.__nightmare !== undefined ||
      win.domAutomation !== undefined ||
      win.domAutomationController !== undefined
    ) {
      return true;
    }
  } catch {
    // no-op
  }
  return false;
}

/** Headless Chrome does not have chrome.runtime by default but has window.chrome */
function probeChromeRuntime(): boolean {
  try {
    const win = window as unknown as { chrome?: { runtime?: unknown } };
    if (win.chrome && !win.chrome.runtime) {
      // Only applicable on desktop Chrome
      const isChromeUA = /Chrome/.test(navigator.userAgent) && !/Mobile/.test(navigator.userAgent);
      return isChromeUA;
    }
  } catch {
    // no-op
  }
  return false;
}

/**
 * Checks if common native functions have been overwritten (spoofed)
 * by stealth frameworks (e.g. puppeteer-extra-plugin-stealth).
 * Overridden functions often fail to hide their non-native string representation.
 */
function probeNativeFunctionSpoofing(): boolean {
  // oxlint-disable-next-line typescript/no-unsafe-function-type -- generic function type for native check
  const checkNative = (fn: Function | undefined): boolean => {
    if (!fn) return false;
    try {
      const str = fn.toString();
      // A native function always serializes to exactly this:
      // "function name() { [native code] }" or similar for async/getters.
      const isNative = str.includes('[native code]') && !str.includes('return');
      return !isNative;
    } catch {
      return true; // threw error on toString = proxy spoofing
    }
  };

  try {
    // Check frequently targetted functions
    if (checkNative(navigator.permissions?.query)) return true;
    if (checkNative(HTMLCanvasElement.prototype.toDataURL)) return true;
    if (checkNative(Function.prototype.toString)) return true;
  } catch {
    return true;
  }
  return false;
}

/** Checks for plugins object spoofing/mocking without relying on deprecated globals directly */
function probePlugins(): boolean {
  try {
    // If navigator.plugins is completely undefined (future standard or sandboxed), it is not a bot indicator.
    if (!navigator.plugins) {
      return false;
    }

    // Stealth frameworks spoof navigator.plugins by mocking it.
    // Verify the native internal [[Class]] brand using Object.prototype.toString.
    const brand = Object.prototype.toString.call(navigator.plugins);
    if (brand !== '[object PluginArray]') {
      return true;
    }

    // If it's a mocked array, they might have spoofed Symbol.toStringTag but still inherit from Array.
    if (Array.isArray(navigator.plugins)) {
      return true;
    }
  } catch {
    return true;
  }
  return false;
}

/**
 * Creates a dynamic iframe to leak the real, un-spoofed navigator.webdriver.
 * High-reliability check: bypasses naive main-frame stealth patches.
 */
function probeIframeWebdriver(): boolean {
  try {
    const iframe = document.createElement('iframe');
    iframe.style.display = 'none';
    // Using srcdoc prevents network access and avoids strict CORS/CSP issues.
    iframe.srcdoc = '';

    document.head.appendChild(iframe);
    const iframeWin = iframe.contentWindow;
    let leakedWebdriver = false;

    if (iframeWin) {
      leakedWebdriver = !!iframeWin.navigator.webdriver;
    }

    document.head.removeChild(iframe);
    return leakedWebdriver;
  } catch {
    // Gracefully handle strict sandboxed environments/WebViews where iframe append is restricted.
    return false;
  }
}

interface UserAgentDataBrand {
  brand: string;
  version: string;
}

interface NavigatorUAData {
  brands: UserAgentDataBrand[];
  mobile: boolean;
  platform: string;
}

/**
 * Verifies consistency between navigator.userAgent and the modern navigator.userAgentData API.
 * Excellent Chromium check; prevents low-effort OS/device spoofing.
 */
function probeUADataInconsistency(): boolean {
  try {
    const uaData = (navigator as unknown as { userAgentData?: NavigatorUAData }).userAgentData;
    if (!uaData?.platform) {
      return false;
    }

    const ua = navigator.userAgent.toLowerCase();
    const platform = uaData.platform.toLowerCase();

    // 1. Platform mismatch
    if (platform.includes('win')) {
      if (!ua.includes('windows') && !ua.includes('win64') && !ua.includes('wow64')) {
        return true;
      }
    } else if (platform.includes('mac')) {
      if (!ua.includes('macintosh') && !ua.includes('mac os')) {
        return true;
      }
    } else if (platform.includes('linux')) {
      if (!ua.includes('linux') && !ua.includes('android')) {
        return true;
      }
    }

    // 2. Mobile status mismatch
    const isMobileUA = /mobi|android|iphone|ipad/i.test(ua);
    if (uaData.mobile !== isMobileUA) {
      return true;
    }
  } catch {
    // no-op
  }
  return false;
}

/**
 * Validates the window.chrome object to detect headless bypasses on Chrome User-Agents.
 * Designed to avoid false positives on non-Chrome environments (Safari, Firefox, native iOS WebViews).
 */
function probeChromeSpoofing(): boolean {
  try {
    const ua = navigator.userAgent;
    const isChrome =
      /Chrome|HeadlessChrome/.test(ua) && !/Edge|Edg|OPR|Firefox|Safari\/[0-9.]+$/.test(ua);

    // Skip if the browser is not desktop Chrome/Chromium to prevent false positives.
    if (!isChrome || /Mobile|Android|iPhone|iPad/i.test(ua)) {
      return false;
    }

    const win = window as unknown as { chrome?: Record<string, unknown> };

    // 1. Desktop Chrome must have window.chrome defined. Headless Chrome doesn't.
    if (win.chrome === undefined) {
      return true;
    }

    // 2. A real Chrome has chrome.runtime defined. Stealth plugins mock window.chrome but forget this.
    if (!win.chrome.runtime) {
      return true;
    }

    // 3. Check property descriptor: In real Chrome, 'chrome' is a standard value property.
    // Stealth frameworks often use defineProperty getters to return a mocked object dynamically.
    const descriptor = Object.getOwnPropertyDescriptor(window, 'chrome');
    if (descriptor && (descriptor.get !== undefined || descriptor.set !== undefined)) {
      return true;
    }
  } catch {
    return true; // proxy tampering errors
  }
  return false;
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

export function collectAutomationSignals(): AutomationSignals {
  const wd = probeWebdriver();

  // Dev Environment Mocking / Debug overrides (only allowed in development environment)
  const isDev =
    typeof import.meta !== 'undefined' &&
    'env' in import.meta &&
    (import.meta as unknown as { env: { DEV: boolean } }).env?.DEV === true;
  const mockAllowed = isDev && typeof localStorage !== 'undefined';

  const mockWebdriver = mockAllowed && localStorage.getItem('__bg_mock_webdriver') === 'true';
  const mockIframeWebdriver =
    mockAllowed && localStorage.getItem('__bg_mock_iframe_webdriver') === 'true';
  const mockChromeSpoofed =
    mockAllowed && localStorage.getItem('__bg_mock_chrome_spoofed') === 'true';
  const mockUAInconsistent =
    mockAllowed && localStorage.getItem('__bg_mock_ua_inconsistent') === 'true';
  const mockNativeSpoofed =
    mockAllowed && localStorage.getItem('__bg_mock_native_spoofed') === 'true';
  const mockChromeRuntimeMissing =
    mockAllowed && localStorage.getItem('__bg_mock_chrome_runtime_missing') === 'true';
  const mockPluginsInconsistent =
    mockAllowed && localStorage.getItem('__bg_mock_plugins_inconsistent') === 'true';
  const mockGlobalToolsDetected =
    mockAllowed && localStorage.getItem('__bg_mock_global_tools_detected') === 'true';
  const mockChromeDriverInjected =
    mockAllowed && localStorage.getItem('__bg_mock_chrome_driver_injected') === 'true';

  return {
    auto_webdriver: mockWebdriver || wd.webdriver,
    auto_webdriver_spoofed: mockWebdriver || wd.spoofed,
    auto_chrome_driver_injected: mockChromeDriverInjected || probeChromeDriver(),
    auto_global_tools_detected: mockGlobalToolsDetected || probeGlobalTools(),
    auto_chrome_runtime_missing: mockChromeRuntimeMissing || probeChromeRuntime(),
    auto_native_function_spoofed: mockNativeSpoofed || probeNativeFunctionSpoofing(),
    auto_plugins_inconsistent: mockPluginsInconsistent || probePlugins(),
    auto_iframe_webdriver: mockIframeWebdriver || probeIframeWebdriver(),
    auto_ua_data_inconsistent: mockUAInconsistent || probeUADataInconsistency(),
    auto_chrome_spoofed: mockChromeSpoofed || probeChromeSpoofing(),
  };
}
