const REDACTED = '[Filtered]';
const SENSITIVE_KEY_PARTS = [
  'authorization',
  'cookie',
  'token',
  'secret',
  'password',
  'jwt',
  'api_key',
  'apikey',
  'private_key',
  'client_secret',
  'session',
  'credential',
  'email',
  'filename',
  'file_name',
  'object_key',
] as const;

const EMAIL_PATTERN = /[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}/gi;
const JWT_PATTERN = /\beyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\b/g;
const BEARER_PATTERN = /\b(Bearer|Basic)\s+[A-Za-z0-9._~+/=-]+/gi;
const SECRET_ASSIGNMENT_PATTERN =
  /\b(token|secret|password|jwt|api[_-]?key|client[_-]?secret)=([^&\s]+)/gi;

export function normalizeServiceWorkerError(error: unknown): Error {
  if (error instanceof Error) {
    return error;
  }

  if (typeof error === 'string') {
    return new Error(scrubSensitiveString(error));
  }

  return new Error('Service worker operation failed');
}

export function scrubUnknownValue(value: unknown, key = ''): unknown {
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
  if (isSensitiveKey(key)) {
    return REDACTED;
  }

  return scrubUnknownValue(value, key);
}

function scrubStringForKey(key: string, value: string): string {
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

function isSensitiveKey(key: string): boolean {
  const normalized = key.toLowerCase().replace(/[^a-z0-9]/g, '_');
  return SENSITIVE_KEY_PARTS.some((part) => normalized.includes(part));
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}
