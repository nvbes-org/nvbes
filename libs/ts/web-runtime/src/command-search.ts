export interface CommandSearchToken {
  key: string;
  value: string;
}

export interface ParsedCommandSearch {
  text: string;
  tokens: CommandSearchToken[];
}

const TOKEN_PATTERN = /(^|\s)([a-z][a-z0-9_-]*):("[^"]*"|'[^']*'|[^\s]+)/giu;

export function parseCommandSearch(value: string): ParsedCommandSearch {
  const tokens: CommandSearchToken[] = [];
  const text = value.replace(TOKEN_PATTERN, (_match, prefix: string, key: string, raw: string) => {
    tokens.push({ key: key.toLowerCase(), value: unquoteTokenValue(raw) });
    return prefix;
  });

  return {
    text: text.replace(/\s+/gu, ' ').trim(),
    tokens,
  };
}

export function commandSearchTokenValue(
  query: ParsedCommandSearch,
  key: string,
): string | undefined {
  return query.tokens.find((token) => token.key === key.toLowerCase())?.value;
}

function unquoteTokenValue(value: string): string {
  if (
    (value.startsWith('"') && value.endsWith('"')) ||
    (value.startsWith("'") && value.endsWith("'"))
  ) {
    return value.slice(1, -1);
  }
  return value;
}
