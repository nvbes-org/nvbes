declare const safeUrlBrand: unique symbol;

export type SafeUrl = string & {
  readonly [safeUrlBrand]: true;
};

export interface SafeUrlOptions {
  allowedProtocols?: readonly string[];
  allowRelative?: boolean;
}

const DEFAULT_ALLOWED_PROTOCOLS = ['https:', 'http:', 'mailto:', 'tel:'] as const;

export function sanitizeUrlForAttribute(
  value: string | null | undefined,
  options: SafeUrlOptions = {},
): SafeUrl | undefined {
  if (!value) {
    return undefined;
  }

  const trimmed = value.trim();
  if (!trimmed || containsEncodedControlChars(trimmed)) {
    return undefined;
  }

  const normalized = removeAsciiWhitespaceAndControls(trimmed).toLowerCase();
  if (
    normalized.startsWith('javascript:') ||
    normalized.startsWith('vbscript:') ||
    normalized.startsWith('data:')
  ) {
    return undefined;
  }

  const allowRelative = options.allowRelative ?? true;
  const allowedProtocols = new Set(options.allowedProtocols ?? DEFAULT_ALLOWED_PROTOCOLS);
  try {
    const parsed = new URL(trimmed, 'https://nvbes.local');
    if (!allowedProtocols.has(parsed.protocol)) {
      return undefined;
    }
    if (!allowRelative && !isAbsoluteUrl(trimmed)) {
      return undefined;
    }
    return trimmed as SafeUrl;
  } catch {
    return undefined;
  }
}

export function safeUrlToString(value: SafeUrl): string {
  return value;
}

function isAbsoluteUrl(value: string): boolean {
  return /^[a-z][a-z\d+\-.]*:/iu.test(value);
}

function removeAsciiWhitespaceAndControls(value: string): string {
  let output = '';
  for (const char of value) {
    const code = char.charCodeAt(0);
    if (code > 0x20 && code !== 0x7f) {
      output += char;
    }
  }
  return output;
}

function containsEncodedControlChars(value: string): boolean {
  return /%0[0-9a-f]|%1[0-9a-f]|%7f/iu.test(value);
}
