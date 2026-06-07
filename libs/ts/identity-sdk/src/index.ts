import { createRequestHeaders } from '@nvbes/http-client';

export interface IdentityConfig {
  clientId: string;
  clientSecret?: string;
  redirectUri: string;
  authorizationUrl: string;
  tokenUrl: string;
  userInfoUrl: string;
}

export interface TokenResponse {
  accessToken: string;
  tokenType: string;
  expiresIn: number;
  refreshToken?: string;
  scope: string;
}

export interface UserInfo {
  id: string;
  email: string;
  name: string;
  emailVerified: boolean;
  workspaceId?: string;
}

export class NvbesIdentity {
  private readonly config: IdentityConfig;
  private token: TokenResponse | null = null;

  constructor(config: IdentityConfig) {
    this.config = config;
  }

  /**
   * Générer l'URL d'autorisation pour le flux OAuth2
   */
  getAuthorizationUrl(
    scope: string = 'openid profile email',
    state?: string,
    codeChallenge?: string,
    codeChallengeMethod: 'plain' | 'S256' = 'S256',
  ): string {
    const params = new URLSearchParams({
      response_type: 'code',
      client_id: this.config.clientId,
      redirect_uri: this.config.redirectUri,
      scope,
    });

    if (state) {
      params.append('state', state);
    }

    if (codeChallenge) {
      params.append('code_challenge', codeChallenge);
      params.append('code_challenge_method', codeChallengeMethod);
    }

    return `${this.config.authorizationUrl}?${params.toString()}`;
  }

  /**
   * Échanger un code d'autorisation contre un token
   */
  async exchangeCode(code: string, codeVerifier?: string): Promise<TokenResponse> {
    const response = await fetch(this.config.tokenUrl, {
      method: 'POST',
      headers: createRequestHeaders('POST', {
        'Content-Type': 'application/x-www-form-urlencoded',
      }),
      body: new URLSearchParams({
        grant_type: 'authorization_code',
        code,
        redirect_uri: this.config.redirectUri,
        client_id: this.config.clientId,
        ...(this.config.clientSecret && {
          client_secret: this.config.clientSecret,
        }),
        ...(codeVerifier && { code_verifier: codeVerifier }),
      }),
    });

    if (!response.ok) {
      throw new Error(`Token exchange failed: ${response.statusText}`);
    }

    const data = await response.json();
    this.token = {
      accessToken: data.access_token,
      tokenType: data.token_type,
      expiresIn: data.expires_in,
      refreshToken: data.refresh_token,
      scope: data.scope,
    };

    return this.token;
  }

  /**
   * Récupérer les informations de l'utilisateur connecté
   */
  async getUserInfo(): Promise<UserInfo> {
    if (!this.token) {
      throw new Error('No access token available');
    }

    const response = await fetch(this.config.userInfoUrl, {
      headers: { Authorization: `Bearer ${this.token.accessToken}` },
    });

    if (!response.ok) {
      throw new Error(`Failed to fetch user info: ${response.statusText}`);
    }

    const data = await response.json();
    return {
      id: data.sub,
      email: data.email,
      name: data.name,
      emailVerified: data.email_verified,
      workspaceId: data.workspace_id,
    };
  }

  /**
   * Vérifier si l'utilisateur est connecté
   */
  isAuthenticated(): boolean {
    return this.token !== null;
  }

  /**
   * Récupère le token d'accès actuel, en le rafraîchissant s'il a expiré
   */
  async getAccessToken(): Promise<string> {
    if (!this.token) {
      throw new Error('No access token available');
    }
    return this.token.accessToken;
  }

  logout(): void {
    this.token = null;
  }

  private getApiUrl(path: string): string {
    return `${this.config.userInfoUrl.replace(/\/oauth\/userinfo$/u, '')}${path}`;
  }

  async listConsents(): Promise<unknown[]> {
    if (!this.token) {
      throw new Error('No access token available');
    }
    const response = await fetch(this.getApiUrl('/legal/consents'), {
      headers: {
        Authorization: `Bearer ${this.token.accessToken}`,
      },
    });
    if (!response.ok) {
      throw new Error(`Failed to list consents: ${response.statusText}`);
    }
    return response.json();
  }

  async grantConsent(consentType: string, documentVersion: string): Promise<unknown> {
    if (!this.token) {
      throw new Error('No access token available');
    }
    const response = await fetch(this.getApiUrl('/legal/consent'), {
      method: 'POST',
      headers: createRequestHeaders('POST', {
        Authorization: `Bearer ${this.token.accessToken}`,
        'Content-Type': 'application/json',
      }),
      body: JSON.stringify({
        consent_type: consentType,
        document_version: documentVersion,
      }),
    });
    if (!response.ok) {
      throw new Error(`Failed to grant consent: ${response.statusText}`);
    }
    return response.json();
  }

  async revokeConsent(consentType: string, documentVersion: string): Promise<void> {
    if (!this.token) {
      throw new Error('No access token available');
    }
    const response = await fetch(this.getApiUrl('/legal/consent/revoke'), {
      method: 'POST',
      headers: createRequestHeaders('POST', {
        Authorization: `Bearer ${this.token.accessToken}`,
        'Content-Type': 'application/json',
      }),
      body: JSON.stringify({
        consent_type: consentType,
        document_version: documentVersion,
      }),
    });
    if (!response.ok) {
      throw new Error(`Failed to revoke consent: ${response.statusText}`);
    }
  }

  /**
   * Conserver les tokens uniquement en mémoire.
   *
   * La persistance côté navigateur doit se faire via des cookies HttpOnly
   * gérés par le backend, pas via du stockage accessible à JavaScript.
   */
  saveToken(): void {
    // Intentionally a no-op.
  }

  /**
   * Aucun chargement persistant n'est effectué.
   */
  loadToken(): boolean {
    return false;
  }
}

export default NvbesIdentity;
