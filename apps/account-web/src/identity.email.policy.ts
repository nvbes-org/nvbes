const EMAIL_FORMAT = /^[^\s@]+@[^\s@]+$/u;

export function emailHasSupportedFormat(value: string): boolean {
  return EMAIL_FORMAT.test(value.trim());
}
