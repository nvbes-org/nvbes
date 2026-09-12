declare const safeStyleElementCssBrand: unique symbol;

export type SafeStyleElementCss = string & {
  readonly [safeStyleElementCssBrand]: true;
};

const UNSAFE_STYLE_ELEMENT_PATTERNS = [
  /<\/\s*style\b/iu,
  /@import\b/iu,
  /\burl\s*\(/iu,
  /\bexpression\s*\(/iu,
  /\bbehavior\s*:/iu,
  /-moz-binding\s*:/iu,
  /javascript\s*:/iu,
  /vbscript\s*:/iu,
  /data\s*:/iu,
];

export function sanitizeStyleElementCss(value: string | null | undefined): SafeStyleElementCss {
  if (!value) {
    return '' as SafeStyleElementCss;
  }

  const normalized = value
    .split('')
    .filter((char) => char.charCodeAt(0) !== 0)
    .join('')
    .trim();
  if (!normalized) {
    return '' as SafeStyleElementCss;
  }

  for (const pattern of UNSAFE_STYLE_ELEMENT_PATTERNS) {
    if (pattern.test(normalized)) {
      return '' as SafeStyleElementCss;
    }
  }

  return normalized as SafeStyleElementCss;
}

export function safeStyleElementCssToString(value: SafeStyleElementCss): string {
  return value;
}
