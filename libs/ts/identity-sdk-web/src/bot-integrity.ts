export interface BotIntegritySignals {
  auto_webdriver: boolean;
  nav_touch_points: number;
  nav_hw_concurrency: number;
  lang: string;
  tz: string;
  tz_offset: number;
  storage_ok: boolean;
  ua_brands?: string[];
  ua_architecture?: string;
  ua_bitness?: string;
  ua_form_factors?: string[];
  ua_full_version_list?: string[];
  ua_model?: string;
  ua_mobile?: boolean;
  ua_platform?: string;
  ua_platform_version?: string;
  ua_wow64?: boolean;
}

type NavigatorUserAgentData = {
  brands: Array<{ brand: string; version: string }>;
  mobile: boolean;
  platform: string;
  getHighEntropyValues?: (hints: string[]) => Promise<Record<string, unknown>>;
};

export async function collectBotIntegritySignals(): Promise<BotIntegritySignals> {
  const signals: BotIntegritySignals = {
    auto_webdriver: navigator.webdriver,
    nav_touch_points: navigator.maxTouchPoints,
    nav_hw_concurrency: navigator.hardwareConcurrency,
    lang: navigator.language,
    tz: Intl.DateTimeFormat().resolvedOptions().timeZone,
    tz_offset: new Date().getTimezoneOffset(),
    storage_ok: storageAvailable(),
  };

  const uaData = (navigator as Navigator & { userAgentData?: NavigatorUserAgentData })
    .userAgentData;
  if (!uaData) return signals;

  signals.ua_mobile = uaData.mobile;
  signals.ua_platform = uaData.platform;
  signals.ua_brands = uaData.brands.map(({ brand, version }) => `${brand}/${version}`);
  if (!uaData.getHighEntropyValues) return signals;

  try {
    const values = await uaData.getHighEntropyValues([
      'architecture',
      'bitness',
      'formFactors',
      'fullVersionList',
      'model',
      'platformVersion',
      'wow64',
    ]);
    assignString(signals, 'ua_architecture', values.architecture);
    assignString(signals, 'ua_bitness', values.bitness);
    assignString(signals, 'ua_model', values.model);
    assignString(signals, 'ua_platform_version', values.platformVersion);
    if (typeof values.wow64 === 'boolean') signals.ua_wow64 = values.wow64;
    if (Array.isArray(values.formFactors)) {
      signals.ua_form_factors = values.formFactors.filter(
        (value): value is string => typeof value === 'string',
      );
    }
    if (Array.isArray(values.fullVersionList)) {
      signals.ua_full_version_list = values.fullVersionList.flatMap((value) => {
        if (!value || typeof value !== 'object') return [];
        const brand = Reflect.get(value, 'brand');
        const version = Reflect.get(value, 'version');
        return typeof brand === 'string' && typeof version === 'string'
          ? [`${brand}/${version}`]
          : [];
      });
    }
  } catch {
    // UA-CH is optional and privacy-budget decisions may reject high-entropy values.
  }
  return signals;
}

function assignString(
  signals: BotIntegritySignals,
  key: 'ua_architecture' | 'ua_bitness' | 'ua_model' | 'ua_platform_version',
  value: unknown,
): void {
  if (typeof value === 'string') signals[key] = value;
}

function storageAvailable(): boolean {
  try {
    const key = '__nvbes_integrity_probe';
    sessionStorage.setItem(key, '1');
    sessionStorage.removeItem(key);
    return true;
  } catch {
    return false;
  }
}
