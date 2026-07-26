export const permissionsPolicy =
  'accelerometer=(), camera=(), geolocation=(), gyroscope=(), magnetometer=(), microphone=(), payment=(), usb=()';
export const integrityPolicyScripts = 'blocked-destinations=(script)';
export const uaClientHintsHeaders = {
  'Accept-CH':
    'Sec-CH-UA, Sec-CH-UA-Arch, Sec-CH-UA-Bitness, Sec-CH-UA-Full-Version, Sec-CH-UA-Full-Version-List, Sec-CH-UA-Model, Sec-CH-UA-WoW64, Sec-CH-UA-Form-Factors, Sec-CH-UA-Mobile, Sec-CH-UA-Platform, Sec-CH-UA-Platform-Version',
} as const;

export interface WebCspOptions {
  mode: string;
  scriptSrc?: string[];
  styleSrc?: string[];
  imgSrc?: string[];
  fontSrc?: string[];
  connectSrc?: string[];
  frameSrc?: string[];
  reportUri?: string;
}

const DEV_CONNECT_SOURCES = [
  'ws://localhost:*',
  'ws://127.0.0.1:*',
  'http://localhost:*',
  'http://127.0.0.1:*',
];

export function buildWebCsp(options: WebCspOptions): string {
  const isDev = options.mode === 'development';
  const reportUri = options.reportUri ?? '/csp-report';
  const directives = [
    directive('default-src', ["'self'"]),
    directive('script-src', [
      "'self'",
      ...(isDev ? ["'unsafe-inline'", "'unsafe-eval'"] : []),
      ...(options.scriptSrc ?? []),
    ]),
    directive('script-src-attr', ["'none'"]),
    directive('worker-src', ["'self'", 'blob:']),
    directive('style-src', [
      "'self'",
      ...(isDev ? ["'unsafe-inline'"] : []),
      ...(options.styleSrc ?? []),
    ]),
    directive('style-src-elem', ["'self'", "'unsafe-inline'", ...(options.styleSrc ?? [])]),
    directive('style-src-attr', ["'unsafe-inline'"]),
    directive('img-src', ["'self'", 'data:', 'blob:', ...(options.imgSrc ?? [])]),
    directive('font-src', ["'self'", 'data:', ...(options.fontSrc ?? [])]),
    directive('connect-src', [
      "'self'",
      ...(isDev ? DEV_CONNECT_SOURCES : []),
      ...(options.connectSrc ?? []),
    ]),
    directive('frame-src', ["'self'", ...(options.frameSrc ?? [])]),
    directive('object-src', ["'none'"]),
    directive('base-uri', ["'self'"]),
    directive('form-action', ["'self'"]),
    directive('frame-ancestors', ["'none'"]),
    reportUri ? directive('report-uri', [reportUri]) : '',
  ].filter(Boolean);

  return `${directives.join('; ')};`;
}

export function cspMetaFromHeader(csp: string): string {
  return csp
    .replace(/;\s*frame-ancestors 'none'/u, '')
    .replace(/;\s*report-uri [^;]+/u, '')
    .replace(/;\s*report-to [^;]+/u, '');
}

export function originFromUrl(value: string): string {
  try {
    return value ? new URL(value).origin : '';
  } catch {
    return '';
  }
}

function directive(name: string, values: string[]): string {
  const uniqueValues = [...new Set(values.filter(Boolean))];
  return `${name} ${uniqueValues.join(' ')}`;
}
