export function browserTimeZone(): string {
  const timezone = Intl.DateTimeFormat().resolvedOptions().timeZone?.trim();
  return timezone || 'UTC';
}
