export const MAX_USERNAME_LENGTH = 100;

export function usernameLength(value: string): number {
  return Array.from(value.trim()).length;
}

export function usernameHasSupportedLength(value: string): boolean {
  const length = usernameLength(value);
  return length > 0 && length <= MAX_USERNAME_LENGTH;
}
