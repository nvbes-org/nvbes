import { useEffect, useMemo, useState } from 'react';

export type RelativeTimeInput = string | number | Date;

export interface RelativeTimeProps {
  value: RelativeTimeInput;
  className?: string;
  locale?: string;
  now?: Date;
  refreshIntervalMs?: number;
}

export function RelativeTime({
  value,
  className,
  locale = 'en',
  now,
  refreshIntervalMs = 60_000,
}: RelativeTimeProps) {
  const [tick, setTick] = useState(0);
  const currentNow = now ?? new Date();
  const absolute = formatAbsoluteDateTime(value, locale);
  const relative = useMemo(
    () => formatRelativeDateTime(value, currentNow, locale),
    [value, currentNow.getTime(), locale, tick],
  );

  useEffect(() => {
    if (now || refreshIntervalMs <= 0) {
      return undefined;
    }
    const timer = window.setInterval(() => setTick((value) => value + 1), refreshIntervalMs);
    return () => window.clearInterval(timer);
  }, [now, refreshIntervalMs]);

  return (
    <time dateTime={toDate(value)?.toISOString()} title={absolute} className={className}>
      {relative}
    </time>
  );
}

export function formatAbsoluteDateTime(value: RelativeTimeInput, locale = 'en'): string {
  const date = toDate(value);
  if (!date) {
    return String(value);
  }

  return new Intl.DateTimeFormat(locale, {
    day: 'numeric',
    hour: 'numeric',
    minute: 'numeric',
    month: 'short',
    second: 'numeric',
    timeZoneName: 'short',
    year: 'numeric',
  }).format(date);
}

export function formatRelativeDateTime(
  value: RelativeTimeInput,
  now: Date = new Date(),
  locale = 'en',
): string {
  const date = toDate(value);
  if (!date) {
    return String(value);
  }

  const seconds = Math.round((date.getTime() - now.getTime()) / 1000);
  const [amount, unit] = relativeUnit(seconds);
  return new Intl.RelativeTimeFormat(locale, { numeric: 'auto' }).format(amount, unit);
}

function toDate(value: RelativeTimeInput): Date | null {
  const date = value instanceof Date ? value : new Date(value);
  return Number.isNaN(date.getTime()) ? null : date;
}

function relativeUnit(seconds: number): [number, Intl.RelativeTimeFormatUnit] {
  const abs = Math.abs(seconds);
  if (abs >= 86_400) {
    return [Math.round(seconds / 86_400), 'day'];
  }
  if (abs >= 3_600) {
    return [Math.round(seconds / 3_600), 'hour'];
  }
  if (abs >= 60) {
    return [Math.round(seconds / 60), 'minute'];
  }
  return [seconds, 'second'];
}
