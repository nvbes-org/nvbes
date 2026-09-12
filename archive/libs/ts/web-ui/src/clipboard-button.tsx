import { Check, Copy } from 'lucide-react';
import { useEffect, useState } from 'react';
import { cn } from './lib/classnames';

export interface ClipboardButtonProps {
  value: string;
  className?: string;
  copiedLabel?: string;
  label?: string;
  resetMs?: number;
}

export function ClipboardButton({
  value,
  className,
  copiedLabel = 'Copied',
  label = 'Copy',
  resetMs = 2_000,
}: ClipboardButtonProps) {
  const [state, setState] = useState<'idle' | 'copied' | 'failed'>('idle');

  useEffect(() => {
    if (state !== 'copied') {
      return undefined;
    }
    const timer = window.setTimeout(() => setState('idle'), resetMs);
    return () => window.clearTimeout(timer);
  }, [resetMs, state]);

  const copy = async () => {
    const copied = await writeClipboardText(value);
    setState(copied ? 'copied' : 'failed');
  };

  return (
    <button
      type="button"
      onClick={() => void copy()}
      className={cn(
        'inline-flex h-8 shrink-0 items-center justify-center gap-1.5 rounded-md border border-border bg-background px-2.5 text-sm font-medium text-foreground transition hover:bg-muted disabled:opacity-50',
        className,
      )}
      aria-label={state === 'copied' ? copiedLabel : label}
      title={state === 'copied' ? copiedLabel : label}
    >
      {state === 'copied' ? <Check className="size-4" /> : <Copy className="size-4" />}
      <span className="sr-only" aria-live="polite">
        {state === 'copied' ? copiedLabel : state === 'failed' ? 'Copy failed' : label}
      </span>
    </button>
  );
}

async function writeClipboardText(value: string): Promise<boolean> {
  if (navigator.clipboard?.writeText) {
    try {
      await navigator.clipboard.writeText(value);
      return true;
    } catch {
      return fallbackCopy(value);
    }
  }

  return fallbackCopy(value);
}

function fallbackCopy(value: string): boolean {
  if (typeof document === 'undefined') {
    return false;
  }

  const textArea = document.createElement('textarea');
  textArea.value = value;
  textArea.setAttribute('readonly', 'true');
  textArea.style.position = 'fixed';
  textArea.style.opacity = '0';
  document.body.append(textArea);
  textArea.select();

  try {
    return document.execCommand('copy');
  } catch {
    return false;
  } finally {
    textArea.remove();
  }
}
