import {
  DEVICE_NAMES,
  type DeviceName,
  OS_NAMES,
  type OperatingSystemName,
  type UserAgentDeviceType,
} from './user-agent.constants';

export type OperatingSystem = readonly [name: OperatingSystemName | null, version: string | null];

export interface DeviceTypeOptions {
  devicePixelRatio?: number;
  maxTouchPoints?: number;
  screenHeight?: number;
  screenWidth?: number;
  userAgentDataPlatform?: string;
}

const windowsVersions: Readonly<Record<string, string>> = {
  '3.51': 'NT 3.11',
  '4.0': 'NT 4.0',
  '5.0': '2000',
  '5.1': 'XP',
  '5.2': 'XP',
  '6.0': 'Vista',
  '6.1': '7',
  '6.2': '8',
  '6.3': '8.1',
  '6.4': '10',
  '10.0': '10',
};

export function detectOS(userAgent: string): OperatingSystem {
  const xbox = /Xbox; Xbox (.*?)[);]/i.exec(userAgent);
  if (xbox) return [OS_NAMES.xbox, xbox[1] ?? null];
  if (/Nintendo/i.test(userAgent)) return [OS_NAMES.nintendo, null];
  if (/PlayStation/i.test(userAgent)) return [OS_NAMES.playStation, null];
  if (/BlackBerry|PlayBook|BB10/i.test(userAgent)) return [OS_NAMES.blackberry, null];

  if (/Windows/i.test(userAgent)) {
    if (/Phone|WPDesktop/.test(userAgent)) return [OS_NAMES.windowsPhone, null];
    if (/Mobile/.test(userAgent) && !/IEMobile\b/.test(userAgent)) {
      return [OS_NAMES.windowsMobile, null];
    }
    const match = /Windows NT ([0-9.]+)/i.exec(userAgent);
    const version = /arm/i.test(userAgent) ? 'RT' : (windowsVersions[match?.[1] ?? ''] ?? null);
    return [OS_NAMES.windows, version];
  }

  const ios = /(?:iPhone|iPad|iPod).*?OS (\d+)_(\d+)_?(\d+)?/.exec(userAgent);
  if (ios) return [OS_NAMES.ios, `${ios[1]}.${ios[2]}.${ios[3] ?? '0'}`];
  if (/iPhone|iPad|iPod/.test(userAgent)) return [OS_NAMES.ios, null];

  const watchOs = /watch.*\/(\d+\.\d+\.\d+)|watch os,(\d+\.\d+),/i.exec(userAgent);
  if (watchOs) return [OS_NAMES.watchOs, watchOs[1] ?? watchOs[2] ?? null];

  const android = /Android (\d+)(?:\.(\d+))?(?:\.(\d+))?/i.exec(userAgent);
  if (android) {
    return [OS_NAMES.android, `${android[1]}.${android[2] ?? '0'}.${android[3] ?? '0'}`];
  }
  if (/Android/i.test(userAgent)) return [OS_NAMES.android, null];

  const macOs = /Mac OS X (\d+)[_.](\d+)[_.]?(\d+)?/i.exec(userAgent);
  if (macOs) return [OS_NAMES.macOs, `${macOs[1]}.${macOs[2]}.${macOs[3] ?? '0'}`];
  if (/Mac/i.test(userAgent)) return [OS_NAMES.macOs, null];
  if (/CrOS/.test(userAgent)) return [OS_NAMES.chromeOs, null];
  if (/Linux|debian/i.test(userAgent)) return [OS_NAMES.linux, null];
  return [null, null];
}

export function detectDevice(userAgent: string): DeviceName | null {
  if (/Nintendo \w+/i.test(userAgent)) return DEVICE_NAMES.nintendo;
  if (/PlayStation \w+/i.test(userAgent)) return DEVICE_NAMES.playStation;
  if (/Xbox/i.test(userAgent)) return DEVICE_NAMES.xbox;
  if (/Ouya/i.test(userAgent)) return DEVICE_NAMES.ouya;
  if (/Windows Phone|WPDesktop/i.test(userAgent)) return DEVICE_NAMES.windowsPhone;
  if (/iPad/.test(userAgent)) return DEVICE_NAMES.iPad;
  if (/iPod/.test(userAgent)) return DEVICE_NAMES.iPodTouch;
  if (/iPhone/.test(userAgent)) return DEVICE_NAMES.iPhone;
  if (/(watch)(?: ?os[,/]|\d,\d\/)[\d.]+/i.test(userAgent)) return DEVICE_NAMES.appleWatch;
  if (/BlackBerry|PlayBook|BB10/i.test(userAgent)) return DEVICE_NAMES.blackberry;
  if (/(kobo)\s(ereader|touch)/i.test(userAgent)) return DEVICE_NAMES.kobo;
  if (/Nokia/i.test(userAgent)) return DEVICE_NAMES.nokia;
  if (
    /(kf[a-z]{2}wi|aeo[c-r]{2})( bui|\))/i.test(userAgent) ||
    /(kf[a-z]+)( bui|\)).+silk\//i.test(userAgent)
  ) {
    return DEVICE_NAMES.kindleFire;
  }
  if (/(Android|ZTE)/i.test(userAgent)) {
    const knownTablet =
      !/Mobile/.test(userAgent) ||
      /(9138B|TB782B|Nexus [97]|pixel c|HUAWEISHT|BTV|noble nook|QTAQZ3)/i.test(userAgent);
    if (knownTablet) return DEVICE_NAMES.androidTablet;
    return DEVICE_NAMES.android;
  }
  if (
    /pixel[\daxl ]{1,6}/i.test(userAgent) ||
    /(huaweimed-al00|tah-|APA|SM-G92|i980|zte|U304AA)/i.test(userAgent) ||
    (/lmy47v/i.test(userAgent) && !/QTAQZ3/i.test(userAgent))
  ) {
    return DEVICE_NAMES.android;
  }
  if (/(pda|Mobile)/i.test(userAgent)) return DEVICE_NAMES.genericMobile;
  if (/Tablet/i.test(userAgent) && !/Tablet pc/i.test(userAgent)) {
    return DEVICE_NAMES.genericTablet;
  }
  return null;
}

export function detectDeviceType(
  userAgent: string,
  options?: DeviceTypeOptions,
): UserAgentDeviceType {
  const device = detectDevice(userAgent);
  if (
    device === DEVICE_NAMES.iPad ||
    device === DEVICE_NAMES.androidTablet ||
    device === DEVICE_NAMES.kobo ||
    device === DEVICE_NAMES.kindleFire ||
    device === DEVICE_NAMES.genericTablet
  ) {
    return 'Tablet';
  }
  if (
    device === DEVICE_NAMES.nintendo ||
    device === DEVICE_NAMES.xbox ||
    device === DEVICE_NAMES.playStation ||
    device === DEVICE_NAMES.ouya
  ) {
    return 'Console';
  }
  if (device === DEVICE_NAMES.appleWatch) return 'Wearable';
  if (device) return 'Mobile';

  if (options?.userAgentDataPlatform === 'Android' && (options.maxTouchPoints ?? 0) > 0) {
    const shortSide = Math.min(options.screenWidth ?? 0, options.screenHeight ?? 0);
    const density = Math.max(options.devicePixelRatio ?? 1, 1);
    return shortSide / density >= 600 ? 'Tablet' : 'Mobile';
  }
  return 'Desktop';
}
