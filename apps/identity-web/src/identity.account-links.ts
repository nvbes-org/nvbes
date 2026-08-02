export function accountWebUrl(path: string): string {
  const baseUrl = import.meta.env.VITE_ACCOUNT_WEB_BASE_URL?.trim();
  if (!baseUrl) {
    throw new Error('VITE_ACCOUNT_WEB_BASE_URL is required.');
  }
  return new URL(path, baseUrl).toString();
}

export function legalDocumentUrl(path: string): string {
  const baseUrl = import.meta.env.VITE_LEGAL_BASE_URL?.trim() || 'https://nvbes.fr';
  return new URL(path, baseUrl).toString();
}
