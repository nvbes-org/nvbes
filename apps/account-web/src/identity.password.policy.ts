export const MIN_PASSWORD_LENGTH = 15;
export const MAX_PASSWORD_LENGTH = 128;

export function passwordCharacterCount(password: string): number {
  return Array.from(password).length;
}

export function passwordHasSupportedLength(password: string): boolean {
  const length = passwordCharacterCount(password);
  return length >= MIN_PASSWORD_LENGTH && length <= MAX_PASSWORD_LENGTH;
}
