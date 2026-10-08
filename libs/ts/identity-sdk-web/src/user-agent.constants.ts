export const BROWSER_NAMES = {
  androidMobile: 'Android Mobile',
  blackberry: 'BlackBerry',
  brave: 'Brave',
  chrome: 'Chrome',
  chromeIos: 'Chrome iOS',
  duckDuckGo: 'DuckDuckGo',
  facebookMobile: 'Facebook Mobile',
  firefox: 'Firefox',
  firefoxIos: 'Firefox iOS',
  googleSearchApp: 'Google Search App',
  internetExplorer: 'Internet Explorer',
  internetExplorerMobile: 'Internet Explorer Mobile',
  konqueror: 'Konqueror',
  microsoftEdge: 'Microsoft Edge',
  mobileSafari: 'Mobile Safari',
  oculusBrowser: 'Oculus Browser',
  opera: 'Opera',
  operaMini: 'Opera Mini',
  paleMoon: 'Pale Moon',
  safari: 'Safari',
  samsungInternet: 'Samsung Internet',
  ucBrowser: 'UC Browser',
  vivaldi: 'Vivaldi',
  waterfox: 'Waterfox',
  whale: 'Whale',
  yandex: 'Yandex',
} as const;

export type BrowserName = (typeof BROWSER_NAMES)[keyof typeof BROWSER_NAMES];

export const DEVICE_NAMES = {
  android: 'Android',
  androidTablet: 'Android Tablet',
  appleWatch: 'Apple Watch',
  blackberry: 'BlackBerry',
  genericMobile: 'Generic mobile',
  genericTablet: 'Generic tablet',
  iPad: 'iPad',
  iPhone: 'iPhone',
  iPodTouch: 'iPod Touch',
  kindleFire: 'Kindle Fire',
  kobo: 'Kobo',
  nintendo: 'Nintendo',
  nokia: 'Nokia',
  ouya: 'Ouya',
  playStation: 'PlayStation',
  windowsPhone: 'Windows Phone',
  xbox: 'Xbox',
} as const;

export type DeviceName = (typeof DEVICE_NAMES)[keyof typeof DEVICE_NAMES];
export type UserAgentDeviceType = 'Console' | 'Desktop' | 'Mobile' | 'Tablet' | 'Wearable';

export const OS_NAMES = {
  android: 'Android',
  blackberry: 'BlackBerry',
  chromeOs: 'Chrome OS',
  ios: 'iOS',
  linux: 'Linux',
  macOs: 'Mac OS X',
  nintendo: 'Nintendo',
  playStation: 'PlayStation',
  watchOs: 'watchOS',
  windows: 'Windows',
  windowsMobile: 'Windows Mobile',
  windowsPhone: 'Windows Phone',
  xbox: 'Xbox',
} as const;

export type OperatingSystemName = (typeof OS_NAMES)[keyof typeof OS_NAMES];
