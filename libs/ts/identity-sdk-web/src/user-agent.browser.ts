import { BROWSER_NAMES, type BrowserName } from './user-agent.constants';

export interface BrowserHints {
  brave?: boolean;
}

export interface BrowserDetectionOptions {
  detectGoogleSearchApp?: boolean;
}

const VERSION = String.raw`(\d+(\.\d+)?)`;
const DEFAULT_VERSION = new RegExp(`Version/${VERSION}`);

function isSafari(userAgent: string): boolean {
  return (
    userAgent.includes('Safari') &&
    !userAgent.includes('Chrome') &&
    !userAgent.includes('Chromium') &&
    !userAgent.includes('Android')
  );
}

export function detectBrowser(
  userAgent: string,
  vendor = '',
  hints?: BrowserHints,
  options?: BrowserDetectionOptions,
): BrowserName | null {
  if (hints?.brave) return BROWSER_NAMES.brave;
  if (options?.detectGoogleSearchApp && userAgent.includes('GSA/')) {
    return BROWSER_NAMES.googleSearchApp;
  }
  if (userAgent.includes(' OPR/') && userAgent.includes('Mini')) {
    return BROWSER_NAMES.operaMini;
  }
  if (userAgent.includes(' OPR/')) return BROWSER_NAMES.opera;
  if (/BlackBerry|PlayBook|BB10/i.test(userAgent)) return BROWSER_NAMES.blackberry;
  if (userAgent.includes('IEMobile') || userAgent.includes('WPDesktop')) {
    return BROWSER_NAMES.internetExplorerMobile;
  }
  if (userAgent.includes('OculusBrowser')) return BROWSER_NAMES.oculusBrowser;
  if (userAgent.includes('SamsungBrowser')) return BROWSER_NAMES.samsungInternet;
  if (/(?:Edge|Edg|EdgA|EdgiOS)\//.test(userAgent)) return BROWSER_NAMES.microsoftEdge;
  if (userAgent.includes('Vivaldi/')) return BROWSER_NAMES.vivaldi;
  if (userAgent.includes('YaBrowser/')) return BROWSER_NAMES.yandex;
  if (userAgent.includes('Whale/')) return BROWSER_NAMES.whale;
  if (userAgent.includes('DuckDuckGo/') || userAgent.includes('Ddg/')) {
    return BROWSER_NAMES.duckDuckGo;
  }
  if (userAgent.includes('FBIOS')) return BROWSER_NAMES.facebookMobile;
  if (userAgent.includes('UCWEB') || userAgent.includes('UCBrowser')) {
    return BROWSER_NAMES.ucBrowser;
  }
  if (userAgent.includes('CriOS')) return BROWSER_NAMES.chromeIos;
  if (
    userAgent.includes('CrMo') ||
    userAgent.includes('Chrome') ||
    userAgent.includes('Chromium')
  ) {
    return BROWSER_NAMES.chrome;
  }
  if (userAgent.includes('Android') && userAgent.includes('Safari')) {
    return BROWSER_NAMES.androidMobile;
  }
  if (userAgent.includes('FxiOS')) return BROWSER_NAMES.firefoxIos;
  if (userAgent.toLowerCase().includes('konqueror')) return BROWSER_NAMES.konqueror;
  if (userAgent.includes('Brave/')) return BROWSER_NAMES.brave;
  if ((vendor.includes('Apple') || isSafari(userAgent)) && userAgent.includes('Mobile')) {
    return BROWSER_NAMES.mobileSafari;
  }
  if (vendor.includes('Apple') || isSafari(userAgent)) return BROWSER_NAMES.safari;
  if (userAgent.includes('PaleMoon/')) return BROWSER_NAMES.paleMoon;
  if (userAgent.includes('Waterfox/')) return BROWSER_NAMES.waterfox;
  if (userAgent.includes('Firefox') || userAgent.includes('Gecko')) {
    return BROWSER_NAMES.firefox;
  }
  if (userAgent.includes('MSIE') || userAgent.includes('Trident/')) {
    return BROWSER_NAMES.internetExplorer;
  }
  return null;
}

const versionPatterns: Partial<Record<BrowserName, readonly RegExp[]>> = {
  [BROWSER_NAMES.internetExplorerMobile]: [new RegExp(`rv:${VERSION}`)],
  [BROWSER_NAMES.microsoftEdge]: [new RegExp(`(?:Edge|Edg|EdgA|EdgiOS)/${VERSION}`)],
  [BROWSER_NAMES.chrome]: [new RegExp(`(?:Chrome|Chromium|CrMo)/${VERSION}`)],
  [BROWSER_NAMES.chromeIos]: [new RegExp(`CriOS/${VERSION}`)],
  [BROWSER_NAMES.ucBrowser]: [new RegExp(`(?:UCBrowser|UCWEB)/${VERSION}`)],
  [BROWSER_NAMES.safari]: [DEFAULT_VERSION],
  [BROWSER_NAMES.mobileSafari]: [DEFAULT_VERSION],
  [BROWSER_NAMES.opera]: [new RegExp(`(?:Opera|OPR)/${VERSION}`)],
  [BROWSER_NAMES.firefox]: [new RegExp(`Firefox/${VERSION}`)],
  [BROWSER_NAMES.firefoxIos]: [new RegExp(`FxiOS/${VERSION}`)],
  [BROWSER_NAMES.konqueror]: [new RegExp(`Konqueror[:/]?${VERSION}`, 'i')],
  [BROWSER_NAMES.blackberry]: [new RegExp(`BlackBerry ${VERSION}`), DEFAULT_VERSION],
  [BROWSER_NAMES.androidMobile]: [new RegExp(`android\\s${VERSION}`, 'i')],
  [BROWSER_NAMES.samsungInternet]: [new RegExp(`SamsungBrowser/${VERSION}`)],
  [BROWSER_NAMES.oculusBrowser]: [new RegExp(`OculusBrowser/${VERSION}`)],
  [BROWSER_NAMES.vivaldi]: [new RegExp(`Vivaldi/${VERSION}`)],
  [BROWSER_NAMES.yandex]: [new RegExp(`YaBrowser/${VERSION}`)],
  [BROWSER_NAMES.whale]: [new RegExp(`Whale/${VERSION}`)],
  [BROWSER_NAMES.brave]: [new RegExp(`Brave/${VERSION}`)],
  [BROWSER_NAMES.duckDuckGo]: [new RegExp(`(?:DuckDuckGo|Ddg)/${VERSION}`)],
  [BROWSER_NAMES.paleMoon]: [new RegExp(`PaleMoon/${VERSION}`)],
  [BROWSER_NAMES.waterfox]: [new RegExp(`Waterfox/${VERSION}`)],
  [BROWSER_NAMES.googleSearchApp]: [new RegExp(`GSA/${VERSION}`)],
  [BROWSER_NAMES.internetExplorer]: [new RegExp(`(?:rv:|MSIE )${VERSION}`)],
};

export function detectBrowserVersion(
  userAgent: string,
  vendor = '',
  hints?: BrowserHints,
  options?: BrowserDetectionOptions,
): number | null {
  const browser = detectBrowser(userAgent, vendor, hints, options);
  if (!browser) return null;

  for (const pattern of versionPatterns[browser] ?? []) {
    const match = userAgent.match(pattern);
    const version = match?.at(-2);
    if (version) return Number.parseFloat(version);
  }
  return null;
}
