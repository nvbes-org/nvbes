import { createRequestHeaders } from '@nvbes/http-client';
import type {
  MfaFactorView,
  RecoveryCodesResult,
  TotpSetupResult,
  UserView,
  WorkspaceView,
} from '@nvbes/identity-sdk-core/src/types';
import { readCurrentAuthuser, readScopedCsrfToken } from './csrf';
import {
  completeWebAuthnStepUp,
  confirmTotp,
  finishWebAuthnRegistration,
  generateRecoveryCodes,
  listMfaFactors,
  registerWebAuthnCredential,
  removeMfaFactor,
  setupTotp,
  startWebAuthnAuthentication,
  startWebAuthnRegistration,
  stepUp,
} from './mfa';
import { generateCodeChallenge, generateCodeVerifier, type PKCEChallenge } from './pkce';
import { memoryStorage, type WebStorage } from './storage';
import type { WebauthnRegistrationKind } from './webauthn';

export interface AuthConfig {
  baseUrl: string;
  clientId: string;
  clientSecret?: string;
  redirectUri: string;
}

export interface IdentityWebConfig extends AuthConfig {
  cookieDomain?: string;
  secureCookies?: boolean;
  storage?: WebStorage;
}

export interface AuthResult {
  user: UserView;
  workspace: WorkspaceView;
}

export class NvbesIdentityWeb {
  private readonly config: IdentityWebConfig;
  private readonly storage: WebStorage;

  constructor(config: IdentityWebConfig) {
    this.config = {
      secureCookies: true,
      ...config,
    };
    this.storage = config.storage ?? memoryStorage;
  }

  async createPKCEChallenge(): Promise<PKCEChallenge> {
    const codeVerifier = generateCodeVerifier();
    const codeChallenge = await generateCodeChallenge(codeVerifier);
    this.storage.saveCodeVerifier(codeVerifier);
    return { codeVerifier, codeChallenge };
  }

  getAuthorizationUrl(options: {
    scope?: string;
    state: string;
    codeChallenge: string;
    nonce: string;
  }): string {
    if (!options.state.trim()) throw new Error('OAuth state is required.');
    if (!options.codeChallenge.trim()) throw new Error('PKCE code challenge is required.');
    if (!options.nonce.trim()) throw new Error('OIDC nonce is required.');

    const params = new URLSearchParams({
      response_type: 'code',
      client_id: this.config.clientId,
      redirect_uri: this.config.redirectUri,
      scope: options.scope ?? 'openid profile email',
      state: options.state,
      code_challenge: options.codeChallenge,
      code_challenge_method: 'S256',
      nonce: options.nonce,
    });
    return `${this.config.baseUrl}/oauth/authorize?${params.toString()}`;
  }

  async redirectToLogin(options: { scope?: string; state: string; nonce: string }): Promise<void> {
    const pkce = await this.createPKCEChallenge();
    window.location.href = this.getAuthorizationUrl({
      scope: options.scope,
      state: options.state,
      codeChallenge: pkce.codeChallenge,
      nonce: options.nonce,
    });
  }

  async getCurrentUser(): Promise<UserView> {
    const authuser = readCurrentAuthuser();
    const response = await fetch(`${this.config.baseUrl}/auth/me`, {
      credentials: 'include',
      headers: authuser ? { 'X-Auth-User': authuser } : undefined,
    });
    if (!response.ok) {
      throw new Error(`Failed to fetch user: ${response.statusText}`);
    }
    return response.json();
  }

  async logout(): Promise<void> {
    const headers: Record<string, string> = {};
    const authuser = readCurrentAuthuser();
    if (authuser) headers['X-Auth-User'] = authuser;

    const csrfToken = readScopedCsrfToken(authuser);
    if (csrfToken) headers['X-CSRF-Token'] = csrfToken;

    await fetch(`${this.config.baseUrl}/auth/logout`, {
      method: 'POST',
      credentials: 'include',
      headers: createRequestHeaders('POST', headers),
    });
  }

  async isAuthenticated(): Promise<boolean> {
    try {
      await this.getCurrentUser();
      return true;
    } catch {
      return false;
    }
  }

  async listMfaFactors(
    token?: string,
    options: { limit?: number; cursor?: string } = {},
  ): Promise<{
    factors: MfaFactorView[];
    mfa_enabled: boolean;
    next_cursor: string | null;
    has_more: boolean;
  }> {
    return listMfaFactors(this.config.baseUrl, token, options);
  }

  async setupTotp(label?: string, token?: string): Promise<TotpSetupResult> {
    return setupTotp(this.config.baseUrl, label, token);
  }

  async confirmTotp(
    factorId: string,
    code: string,
    token?: string,
  ): Promise<{ factor: MfaFactorView; mfa_enabled: boolean }> {
    return confirmTotp(this.config.baseUrl, factorId, code, token);
  }

  async startWebAuthnRegistration(
    label?: string,
    kind: WebauthnRegistrationKind = 'passkey',
    token?: string,
  ): Promise<{ factorId: string; options: PublicKeyCredentialCreationOptions }> {
    return startWebAuthnRegistration(this.config.baseUrl, label, kind, token);
  }

  async finishWebAuthnRegistration(
    factorId: string,
    credential: PublicKeyCredential,
    token?: string,
  ): Promise<void> {
    return finishWebAuthnRegistration(this.config.baseUrl, factorId, credential, token);
  }

  async registerWebAuthnCredential(
    label?: string,
    kind: WebauthnRegistrationKind = 'passkey',
    token?: string,
  ): Promise<void> {
    return registerWebAuthnCredential(this.config.baseUrl, label, kind, token);
  }

  async startWebAuthnAuthentication(
    token?: string,
  ): Promise<{ challengeId: string; options: PublicKeyCredentialRequestOptions }> {
    return startWebAuthnAuthentication(this.config.baseUrl, token);
  }

  async completeWebAuthnStepUp(token?: string): Promise<void> {
    return completeWebAuthnStepUp(this.config.baseUrl, token);
  }

  async generateRecoveryCodes(password?: string, token?: string): Promise<RecoveryCodesResult> {
    return generateRecoveryCodes(this.config.baseUrl, password, token);
  }

  async removeMfaFactor(factorId: string, token?: string): Promise<void> {
    return removeMfaFactor(this.config.baseUrl, factorId, token);
  }

  async stepUp(
    credentials: {
      password?: string;
      totpCode?: string;
      webauthnResponse?: unknown;
      webauthnChallengeId?: string;
      recoveryCode?: string;
    },
    token?: string,
  ): Promise<{ success: boolean; valid_until: string }> {
    return stepUp(this.config.baseUrl, credentials, token);
  }
}

export default NvbesIdentityWeb;
