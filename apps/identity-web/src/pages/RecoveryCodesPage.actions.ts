export async function generateCodes(password = '') {
  const { generateRecoveryCodes } = await import('@nvbes/identity-sdk-web');
  return generateRecoveryCodes('', password);
}

export function downloadRecoveryCodes(codes: string[]) {
  const blob = new Blob([codes.join('\n')], { type: 'text/plain' });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement('a');
  anchor.href = url;
  anchor.download = 'nvbes-recovery-codes.txt';
  anchor.click();
  URL.revokeObjectURL(url);
}
