import { createRequestHeaders, HttpError } from '@nvbes/http-client';
import type {
  MfaFactorView,
  RecoveryCodesResult,
  TotpSetupResult,
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
import {
  exchangeAuthorizationCode,
  type AuthorizationCodeTokenResponse,
} from './oauth.authorization-code';
import {
  createAuthorizationRequest,
  type AuthorizationRequest,
  type AuthorizationRequestInput,
} from './oauth.authorization-request';
import { defaultWebStorage, type WebStorage } from './storage';
import type { WebauthnRegistrationKind } from './webauthn';

export interface AuthConfig {
  baseUrl: string;
  clientId: string;
  redirectUri: string;
  audience?: string;
}

export interface IdentityWebConfig extends AuthConfig {
  cookieDomain?: string;
  secureCookies?: boolean;
  storage?: WebStorage;
}

export class NvbesIdentityWeb {
  private readonly config: IdentityWebConfig;
  private readonly storage: WebStorage;

  constructor(config: IdentityWebConfig) {
    this.config = {
      secureCookies: true,
      ...config,
    };
    this.storage = config.storage ?? defaultWebStorage();
  }

  createAuthorizationRequest(
    options: AuthorizationRequestInput = {},
  ): Promise<AuthorizationRequest> {
    return createAuthorizationRequest(
      {
        baseUrl: this.config.baseUrl,
        clientId: this.config.clientId,
        redirectUri: this.config.redirectUri,
        audience: this.config.audience,
        storage: this.storage,
      },
      options,
    );
  }

  async redirectToLogin(options: AuthorizationRequestInput = {}): Promise<void> {
    const request = await this.createAuthorizationRequest(options);
    window.location.assign(request.authorizationUrl);
  }

  exchangeAuthorizationCode(input: {
    code: string;
    state: string;
  }): Promise<AuthorizationCodeTokenResponse> {
    return exchangeAuthorizationCode(
      {
        baseUrl: this.config.baseUrl,
        clientId: this.config.clientId,
        redirectUri: this.config.redirectUri,
        storage: this.storage,
      },
      input,
    );
  }

  clearAuthorizationTransaction(): void {
    this.storage.clearTransaction();
  }

  async logout(): Promise<void> {
    const headers: Record<string, string> = {};
    const authuser = readCurrentAuthuser();
    if (authuser) headers['X-Auth-User'] = authuser;

    const csrfToken = readScopedCsrfToken(authuser);
    if (csrfToken) headers['X-CSRF-Token'] = csrfToken;

    const response = await fetch(`${this.config.baseUrl}/auth/logout`, {
      method: 'POST',
      credentials: 'include',
      headers: createRequestHeaders('POST', headers),
    });
    if (!response.ok) {
      throw new HttpError('Identity logout failed', response, undefined);
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
