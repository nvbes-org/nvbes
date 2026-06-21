import type { ReactNode } from 'react';

export interface InvisibleUnicodeMatch {
  index: number;
  char: string;
  codePoint: string;
  name: string;
}

const INVISIBLE_UNICODE_PATTERN =
  /\u{00ad}|\u{034f}|\u{061c}|\u{115f}|\u{1160}|\u{17b4}|\u{17b5}|\u{180e}|[\u{200b}-\u{200f}]|[\u{202a}-\u{202e}]|[\u{2060}-\u{206f}]|\u{feff}|[\u{fff9}-\u{fffb}]/gu;

const INVISIBLE_UNICODE_NAMES: Record<string, string> = {
  '00ad': 'soft hyphen',
  '034f': 'combining grapheme joiner',
  '061c': 'arabic letter mark',
  '180e': 'mongolian vowel separator',
  '200b': 'zero width space',
  '200c': 'zero width non-joiner',
  '200d': 'zero width joiner',
  '200e': 'left-to-right mark',
  '200f': 'right-to-left mark',
  '202a': 'left-to-right embedding',
  '202b': 'right-to-left embedding',
  '202c': 'pop directional formatting',
  '202d': 'left-to-right override',
  '202e': 'right-to-left override',
  '2060': 'word joiner',
  '2061': 'function application',
  '2062': 'invisible times',
  '2063': 'invisible separator',
  '2064': 'invisible plus',
  '2066': 'left-to-right isolate',
  '2067': 'right-to-left isolate',
  '2068': 'first strong isolate',
  '2069': 'pop directional isolate',
  feff: 'zero width no-break space',
};

export function findInvisibleUnicodeCharacters(value: string): InvisibleUnicodeMatch[] {
  const matches: InvisibleUnicodeMatch[] = [];
  for (const match of value.matchAll(INVISIBLE_UNICODE_PATTERN)) {
    const char = match[0] ?? '';
    const code = char.codePointAt(0)?.toString(16).padStart(4, '0') ?? '0000';
    matches.push({
      char,
      codePoint: `U+${code.toUpperCase()}`,
      index: match.index ?? 0,
      name: INVISIBLE_UNICODE_NAMES[code.toLowerCase()] ?? 'invisible unicode control',
    });
  }
  return matches;
}

export function hasInvisibleUnicodeCharacters(value: string): boolean {
  return findInvisibleUnicodeCharacters(value).length > 0;
}

export function revealInvisibleUnicodeCharacters(value: string): string {
  return value.replace(INVISIBLE_UNICODE_PATTERN, (char) => {
    const code = char.codePointAt(0)?.toString(16).padStart(4, '0').toUpperCase() ?? '0000';
    return `[U+${code}]`;
  });
}

export function InvisibleUnicodeWarning({
  value,
  label = 'Hidden Unicode characters detected',
  children,
}: {
  value: string;
  label?: string;
  children?: ReactNode;
}) {
  const matches = findInvisibleUnicodeCharacters(value);
  if (matches.length === 0) {
    return null;
  }

  return (
    <div className="rounded-md border border-amber-500/30 bg-amber-500/10 p-3 text-sm text-amber-900">
      <p className="font-medium">{label}</p>
      <p className="mt-1 text-xs">
        {matches.length} invisible or bidirectional control character
        {matches.length === 1 ? '' : 's'} may change how this value is read.
      </p>
      <details className="mt-2">
        <summary className="cursor-pointer text-xs font-medium">Reveal characters</summary>
        <pre className="mt-2 max-h-32 overflow-auto rounded border border-amber-500/20 bg-background p-2 text-xs text-foreground">
          {revealInvisibleUnicodeCharacters(value)}
        </pre>
      </details>
      {children}
    </div>
  );
}
