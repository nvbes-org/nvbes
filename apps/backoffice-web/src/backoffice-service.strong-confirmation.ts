export function strongConfirmationCode(baseCode: string, targetId: string): string {
  const compactTargetId = targetId.trim().replaceAll('-', '');
  if (compactTargetId.length < 8) return baseCode;
  return `${baseCode} ${compactTargetId.slice(0, 8).toUpperCase()}`;
}
