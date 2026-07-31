export interface AccountOAuthCallbackParams {
  code: string;
  state: string;
}

export function parseAccountOAuthCallback(search: string): AccountOAuthCallbackParams {
  const params = new URLSearchParams(search);
  const oauthError = params.get('error');
  if (oauthError) {
    throw new Error(params.get('error_description') || oauthError);
  }

  const code = params.get('code');
  const state = params.get('state');
  if (!code || !state) {
    throw new Error('Identity a retourné une réponse OAuth incomplète.');
  }
  return { code, state };
}

export function assertGrantedAccountScopes(scope: string, requiredScopes: readonly string[]): void {
  const grantedScopes = new Set(scope.split(/\s+/u).filter(Boolean));
  const missingScope = requiredScopes.find((requiredScope) => !grantedScopes.has(requiredScope));
  if (missingScope) {
    throw new Error(`Identity n'a pas accordé le scope requis ${missingScope}.`);
  }
}
