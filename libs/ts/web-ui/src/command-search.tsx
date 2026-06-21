import { Search } from 'lucide-react';
import { useMemo, useState } from 'react';
import { cn } from './lib/classnames';

export interface CommandSearchToken {
  key: string;
  description: string;
  example: string;
}

export interface CommandSearchProps {
  value: string;
  tokens: CommandSearchToken[];
  onChange: (value: string) => void;
  className?: string;
  placeholder?: string;
}

export function CommandSearch({
  value,
  tokens,
  onChange,
  className,
  placeholder = 'Search...',
}: CommandSearchProps) {
  const [focused, setFocused] = useState(false);
  const filteredTokens = useMemo(() => filterTokens(tokens, value), [tokens, value]);

  return (
    <div className={cn('relative', className)}>
      <Search className="pointer-events-none absolute top-2.5 left-3 size-4 text-muted-foreground" />
      <input
        value={value}
        onChange={(event) => onChange(event.currentTarget.value)}
        onFocus={() => setFocused(true)}
        onBlur={() => window.setTimeout(() => setFocused(false), 100)}
        className="h-10 w-full rounded-md border border-input bg-background pr-3 pl-9 text-sm"
        placeholder={placeholder}
      />
      {focused && filteredTokens.length > 0 ? (
        <div className="absolute top-11 left-0 z-50 w-full rounded-md border border-border bg-popover p-2 shadow-lg">
          {filteredTokens.map((token) => (
            <button
              key={token.key}
              type="button"
              className="flex w-full items-start justify-between gap-3 rounded px-2 py-1.5 text-left hover:bg-muted"
              onMouseDown={(event) => event.preventDefault()}
              onClick={() => onChange(insertToken(value, token.example))}
            >
              <span>
                <span className="font-mono text-xs font-semibold">{token.key}:</span>
                <span className="ml-2 text-xs text-muted-foreground">{token.description}</span>
              </span>
              <span className="font-mono text-xs text-muted-foreground">{token.example}</span>
            </button>
          ))}
        </div>
      ) : null}
    </div>
  );
}

function filterTokens(tokens: CommandSearchToken[], value: string): CommandSearchToken[] {
  const lastWord = value.split(/\s/u).at(-1)?.toLowerCase() ?? '';
  if (!lastWord || lastWord.includes(':')) {
    return tokens;
  }
  return tokens.filter((token) => token.key.startsWith(lastWord));
}

function insertToken(value: string, example: string): string {
  const parts = value.trimEnd().split(/\s/u);
  const last = parts.at(-1) ?? '';
  if (last && !last.includes(':')) {
    parts[parts.length - 1] = example;
    return `${parts.join(' ')} `;
  }
  return `${value.trimEnd()} ${example} `.trimStart();
}
