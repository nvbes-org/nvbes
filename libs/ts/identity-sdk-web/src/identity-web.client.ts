import { logoutHostedSession } from './hosted.client';
import type {
  MfaFactorView,
  RecoveryCodesResult,
  TotpSetupResult,
} from '@nvbes/identity-sdk-core/src/types';
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
import type { DpopTransactionStore } from './dpop.transaction-store';

export interface AuthConfig {
  baseUrl: string;
  clientId: string;
  redirectUri: string;
  resource: string;
}

export interface IdentityWebConfig extends AuthConfig {
  cookieDomain?: string;
  secureCookies?: boolean;
  storage?: WebStorage;
  dpop?: boolean;
  dpopStore?: DpopTransactionStore;
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
        resource: this.config.resource,
        dpop: this.config.dpop,
        dpopStore: this.config.dpopStore,
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
        dpopStore: this.config.dpopStore,
      },
      input,
    );
  }

  clearAuthorizationTransaction(): void {
    this.storage.clearTransaction();
  }

  async logout(sessionCsrfToken?: string): Promise<void> {
    if (!sessionCsrfToken)
      throw new Error('Identity logout requires an explicit session CSRF token.');
    await logoutHostedSession({ baseUrl: this.config.baseUrl }, sessionCsrfToken);
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
