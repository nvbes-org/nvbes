/**
 * Bot Guard — Browser Extensions and Behavioral Abuses Detector
 *
 * Detects presence of blacklisted/abusive browser extensions (scrapers,
 * automators, test tools) while explicitly distinguishing and authorizing
 * common adblockers to avoid false-positive bot scoring.
 */

export interface ExtensionSignals {
  /** True if any blacklisted/fraudulent extension is detected */
  ext_blacklisted_detected: boolean;
  /** List of detected blacklisted extension IDs or names */
  ext_blacklisted_list: string[];
  /** True if any adblocker is active (implicitly authorized, does not increase bot score) */
  ext_adblocker_active: boolean;
  /** List of detected adblockers */
  ext_adblocker_list: string[];
  /** True if DOM has suspicious modifications typical of scraping or auto-filling */
  ext_dom_tampered: boolean;
}

// ---------------------------------------------------------------------------
// Detection Config
// ---------------------------------------------------------------------------

interface ExtensionDef {
  id: string;
  name: string;
  resource: string;
  isBlacklisted: boolean;
}

const EXTENSION_REGISTRY: ExtensionDef[] = [
  // ---- Blacklisted / Abusive / Suspicious Extensions ----
  {
    id: 'mooikpehlononcheghgjoeegainddaak',
    name: 'Selenium IDE',
    resource: 'icons/icon-16.png',
    isBlacklisted: true,
  },
  {
    id: 'ljdobackegjjeeajcblmcmokngeoecne',
    name: 'Katalon Recorder',
    resource: 'icons/katalon-16.png',
    isBlacklisted: true,
  },
  {
    id: 'jnhgnmkkplepadonbbokglneebgofinf',
    name: 'Web Scraper',
    resource: 'assets/images/logo.png',
    isBlacklisted: true,
  },
  {
    id: 'nlmmbnhpmkiiljbeeonifphmainnnnkb',
    name: 'AutoFill',
    resource: 'images/icon16.png',
    isBlacklisted: true,
  },

  // ---- Adblockers (Explicitly Authorized) ----
  {
    id: 'cjpalhdlnbpafiamejdnhcphjbkeiagm',
    name: 'uBlock Origin',
    resource: 'img/icon128.png',
    isBlacklisted: false,
  },
  {
    id: 'cfhdojbgaafkabcipgbacedgikghhhjm',
    name: 'Adblock Plus',
    resource: 'icons/adblockplus32.png',
    isBlacklisted: false,
  },
  {
    id: 'gighmmpiobklfepjocnamgkkbiglidom',
    name: 'AdBlock',
    resource: 'images/icon24.png',
    isBlacklisted: false,
  },
  {
    id: 'mlomiejdfkolichcflejhmhhcbbndejc',
    name: 'Ghostery',
    resource: 'images/ghosty.svg',
    isBlacklisted: false,
  },
];

// ---------------------------------------------------------------------------
// Probes
// ---------------------------------------------------------------------------

/** Probes a single extension by attempting to fetch its Web Accessible Resource */
async function probeExtensionResource(ext: ExtensionDef): Promise<boolean> {
  const url = `chrome-extension://${ext.id}/${ext.resource}`;
  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), 120); // strict short timeout to prevent blocking

  try {
    const res = await fetch(url, {
      method: 'HEAD',
      mode: 'no-cors',
      signal: controller.signal,
    });
    clearTimeout(timeoutId);
    // mode 'no-cors' resolves status to 0, but indicates the resource is fetchable and accessible
    return res.status === 0 || res.status === 200;
  } catch {
    clearTimeout(timeoutId);
    return false;
  }
}

/** Detects active adblocker through DOM advertisement decoy blocking behavior */
function probeAdblockerDOM(): boolean {
  try {
    // Adblockers use strict selector lists to hide elements with specific ad classes or IDs.
    const adDecoy = document.createElement('div');
    adDecoy.className =
      'pub_300x250 pub_300x250m pub_728x90 text-ad textAd text_ad text_ads text-ads ad-banner';
    adDecoy.setAttribute(
      'style',
      'position: absolute; top: -9999px; left: -9999px; width: 1px; height: 1px;',
    );
    document.body.appendChild(adDecoy);

    const computedStyle = window.getComputedStyle(adDecoy);
    const isBlocked =
      computedStyle.display === 'none' ||
      computedStyle.visibility === 'hidden' ||
      adDecoy.offsetHeight === 0;

    document.body.removeChild(adDecoy);
    return isBlocked;
  } catch {
    return false;
  }
}

/** Checks for suspicious DOM tampering or auto-filling injected parameters */
function probeDOMTampering(): boolean {
  try {
    // 1. Scrapers often inject custom helper elements or markers
    const suspiciousSelectors = [
      '[data-webscraper-active]',
      '[data-autofill-active]',
      '.selenium-injected',
      '#selenium-ide-indicator',
    ];
    for (const selector of suspiciousSelectors) {
      if (document.querySelector(selector) !== null) {
        return true;
      }
    }

    // 2. Look for inputs having specific automation-injected classes or attributes
    const inputs = document.querySelectorAll('input');
    for (let i = 0; i < inputs.length; i++) {
      const input = inputs[i];
      if (
        input.getAttribute('data-autofill') === 'true' ||
        input.classList.contains('autofilled-by-extension')
      ) {
        return true;
      }
    }
  } catch {
    // no-op
  }
  return false;
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

export async function collectExtensionSignals(): Promise<ExtensionSignals> {
  const isChromeOrFirefox = /Chrome|Firefox/.test(navigator.userAgent);

  const blacklistedList: string[] = [];
  const adblockerList: string[] = [];

  // Probe extensions by resource fetching if supported browser
  if (isChromeOrFirefox) {
    const probes = EXTENSION_REGISTRY.map(async (ext) => {
      const detected = await probeExtensionResource(ext);
      if (detected) {
        if (ext.isBlacklisted) {
          blacklistedList.push(ext.name);
        } else {
          adblockerList.push(ext.name);
        }
      }
    });
    await Promise.all(probes);
  }

  // Also check behaviorally for Adblock (DOM decoy check)
  const adblockerDOMActive = probeAdblockerDOM();
  if (adblockerDOMActive && !adblockerList.includes('Generic Adblocker')) {
    adblockerList.push('Generic Adblocker');
  }

  const domTampered = probeDOMTampering();

  // Dev Mock/Debug Environment overrides (only allowed in development environment)
  const isDev =
    typeof import.meta !== 'undefined' &&
    'env' in import.meta &&
    (import.meta as unknown as { env: { DEV: boolean } }).env?.DEV === true;
  const mockAllowed = isDev && typeof localStorage !== 'undefined';

  const mockBlacklisted =
    mockAllowed && localStorage.getItem('__bg_mock_ext_blacklisted') === 'true';
  const mockAdblocker = mockAllowed && localStorage.getItem('__bg_mock_ext_adblocker') === 'true';
  const mockDomTampered =
    mockAllowed && localStorage.getItem('__bg_mock_ext_dom_tampered') === 'true';

  return {
    ext_blacklisted_detected: mockBlacklisted || blacklistedList.length > 0,
    ext_blacklisted_list: mockBlacklisted
      ? ['Selenium IDE (Mocked)', 'Web Scraper (Mocked)']
      : blacklistedList,
    ext_adblocker_active: mockAdblocker || adblockerList.length > 0,
    ext_adblocker_list: mockAdblocker ? ['uBlock Origin (Mocked)'] : adblockerList,
    ext_dom_tampered: mockDomTampered || domTampered,
  };
}
