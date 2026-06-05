const REDACTED = '[Filtered]';
const REDACTED_PATH_SEGMENT = '[id]';

const SENSITIVE_KEY_PARTS = [
  'authorization',
  'cookie',
  'token',
  'secret',
  'password',
  'passwd',
  'pwd',
  'jwt',
  'api_key',
  'apikey',
  'private_key',
  'client_secret',
  'session',
  'refresh_token',
  'access_token',
  'id_token',
  'saml',
  'assertion',
  'credential',
  'mfa',
  'totp',
  'recovery',
  'email',
  'username',
  'file_name',
  'filename',
  'object_key',
  'objectkey',
] as const;

const SAFE_HEADER_KEYS = new Set(['accept', 'content_length', 'content_type', 'user_agent']);

const EMAIL_PATTERN = /[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}/gi;
const JWT_PATTERN = /\beyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\b/g;
const BEARER_PATTERN = /\b(Bearer|Basic)\s+[A-Za-z0-9._~+/=-]+/gi;
const SECRET_ASSIGNMENT_PATTERN =
  /\b(token|secret|password|passwd|pwd|jwt|api[_-]?key|client[_-]?secret)=([^&\s]+)/gi;
const UUID_PATH_SEGMENT_PATTERN =
  /^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;
const LONG_TOKEN_PATH_SEGMENT_PATTERN = /^[A-Za-z0-9_-]{24,}$/;
const STATIC_ASSET_PATTERN = /\.(?:css|js|mjs|map|png|jpg|jpeg|gif|svg|ico|webp|avif|woff|woff2)$/i;
const FILE_EXTENSION_PATTERN = /\.[a-z0-9]{1,12}$/i;

export function createSentryReplayPrivacyOptions() {
  return {
    maskAllText: true,
    maskAllInputs: true,
    blockAllMedia: true,
    networkDetailAllowUrls: [],
    networkDetailDenyUrls: [/.*/],
    networkCaptureBodies: false,
    networkRequestHeaders: [],
    networkResponseHeaders: [],
    beforeAddRecordingEvent: scrubReplayRecordingEvent,
  };
}

export function getSentryTracesSampleRate(isProduction: boolean): number {
  return isProduction ? 0.2 : 1.0;
}

export function getSentryReplaysOnErrorSampleRate(isProduction: boolean): number {
  return isProduction ? 0.1 : 1.0;
}

export function scrubSentryEvent<T>(event: T): T | null {
  const scrubbed = scrubUnknownValue(event);
  if (!isRecord(scrubbed)) {
    return scrubbed as T;
  }

  delete scrubbed.user;

  const request = scrubbed.request;
  if (isRecord(request)) {
    const scrubbedRequest = { ...request };

    if (typeof scrubbedRequest.url === 'string') {
      scrubbedRequest.url = sanitizeUrlString(scrubbedRequest.url);
    }

    delete scrubbedRequest.query_string;
    delete scrubbedRequest.cookies;
    delete scrubbedRequest.data;
    scrubbedRequest.headers = scrubHeaderRecord(scrubbedRequest.headers);
    scrubbedRequest.env = scrubHeaderRecord(scrubbedRequest.env);
    scrubbed.request = scrubbedRequest;
  }

  return scrubbed as T;
}

export function scrubSentryBreadcrumb<
  T extends { category?: string; message?: string; data?: unknown },
>(breadcrumb: T): T | null {
  if (breadcrumb.category?.startsWith('console')) {
    return null;
  }

  return scrubUnknownValue(breadcrumb) as T;
}

export function scrubReplayRecordingEvent<T>(event: T): T | null {
  if (isReplayConsoleFrame(event)) {
    return null;
  }

  return scrubUnknownValue(event) as T;
}

export function sanitizeUrlString(value: string): string {
  try {
    const hasExplicitOrigin = /^[a-z][a-z0-9+.-]*:/i.test(value);
    const parsed = new URL(value, 'https://nvbes.invalid');

    parsed.username = '';
    parsed.password = '';
    parsed.hash = '';
    parsed.pathname = sanitizePathname(parsed.pathname);

    for (const key of Array.from(parsed.searchParams.keys())) {
      parsed.searchParams.set(key, REDACTED);
    }

    if (!hasExplicitOrigin) {
      return `${parsed.pathname}${parsed.search}`;
    }

    return parsed.toString();
  } catch {
    return scrubSensitiveString(value);
  }
}

function scrubUnknownValue(value: unknown, key = ''): unknown {
  if (typeof value === 'string') {
    return scrubStringForKey(key, value);
  }

  if (Array.isArray(value)) {
    return value.map((item) => scrubUnknownValue(item, key));
  }

  if (!isRecord(value)) {
    return value;
  }

  const scrubbed: Record<string, unknown> = {};
  for (const [childKey, childValue] of Object.entries(value)) {
    scrubbed[childKey] = scrubValueForKey(childKey, childValue);
  }
  return scrubbed;
}

function scrubValueForKey(key: string, value: unknown): unknown {
  if (typeof value === 'string' && isUrlKey(key)) {
    return sanitizeUrlString(value);
  }

  if (isSensitiveKey(key)) {
    return REDACTED;
  }

  return scrubUnknownValue(value, key);
}

function scrubStringForKey(key: string, value: string): string {
  if (isUrlKey(key)) {
    return sanitizeUrlString(value);
  }

  if (isSensitiveKey(key)) {
    return REDACTED;
  }

  return scrubSensitiveString(value);
}

function scrubSensitiveString(value: string): string {
  return value
    .replace(EMAIL_PATTERN, REDACTED)
    .replace(JWT_PATTERN, REDACTED)
    .replace(BEARER_PATTERN, (_match, scheme: string) => `${scheme} ${REDACTED}`)
    .replace(SECRET_ASSIGNMENT_PATTERN, (_match, key: string) => `${key}=${REDACTED}`);
}

function scrubHeaderRecord(value: unknown): Record<string, string> {
  if (!isRecord(value)) {
    return {};
  }

  const scrubbed: Record<string, string> = {};
  for (const [key, headerValue] of Object.entries(value)) {
    const normalizedKey = normalizeKey(key);
    if (!SAFE_HEADER_KEYS.has(normalizedKey) || isSensitiveKey(key)) {
      continue;
    }

    if (typeof headerValue === 'string') {
      scrubbed[key] = scrubSensitiveString(headerValue);
    }
  }

  return scrubbed;
}

function sanitizePathname(pathname: string): string {
  return pathname
    .split('/')
    .map((segment) => (shouldRedactPathSegment(segment) ? REDACTED_PATH_SEGMENT : segment))
    .join('/');
}

function shouldRedactPathSegment(segment: string): boolean {
  if (!segment) {
    return false;
  }

  const decoded = decodeURIComponentSafe(segment);
  if (decoded.includes('@')) {
    return true;
  }

  if (UUID_PATH_SEGMENT_PATTERN.test(decoded) || LONG_TOKEN_PATH_SEGMENT_PATTERN.test(decoded)) {
    return true;
  }

  return FILE_EXTENSION_PATTERN.test(decoded) && !STATIC_ASSET_PATTERN.test(decoded);
}

function decodeURIComponentSafe(value: string): string {
  try {
    return decodeURIComponent(value);
  } catch {
    return value;
  }
}

function isReplayConsoleFrame(value: unknown): boolean {
  if (!isRecord(value) || !isRecord(value.data)) {
    return false;
  }

  const payload = value.data.payload;
  if (!isRecord(payload)) {
    return false;
  }

  return payload.category === 'console' || payload.type === 'console';
}

function isSensitiveKey(key: string): boolean {
  const normalized = normalizeKey(key);
  return SENSITIVE_KEY_PARTS.some(
    (part) =>
      normalized === part || normalized.endsWith(`_${part}`) || normalized.includes(`${part}_`),
  );
}

function isUrlKey(key: string): boolean {
  const normalized = normalizeKey(key);
  return normalized === 'url' || normalized === 'href' || normalized.endsWith('_url');
}

function normalizeKey(key: string): string {
  return key.replace(/[.\-\s]/g, '_').toLowerCase();
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}
