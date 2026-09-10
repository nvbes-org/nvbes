import { logoutHostedSession } from './hosted.client';
import {
  listHostedTotpFactors,
  revokeHostedTotpFactor,
  type HostedTotpFactor,
} from './hosted.totp.management';
import {
  startHostedTotpEnrollment,
  confirmHostedTotpEnrollment,
  stepUpHostedTotp,
  type HostedTotpEnrollment,
} from './hosted.totp';
import type { HostedInteraction } from './hosted.client';
import { registerHostedPasskey, loginHostedPasskey, stepUpHostedPasskey } from './hosted.webauthn';
import {
  listHostedPasskeys,
  renameHostedPasskey,
  revokeHostedPasskey,
  type HostedPasskey,
} from './hosted.webauthn.credentials';
import type { MfaFactorView, RecoveryCodesResult } from '@nvbes/identity-sdk-core/src/types';
import { generateRecoveryCodes, listMfaFactors, removeMfaFactor, stepUp } from './mfa';
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
import type { WebauthnCreateOptions, WebauthnGetOptions } from './webauthn';
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

  startTotpEnrollment(sessionCsrf: string): Promise<HostedTotpEnrollment> {
    return startHostedTotpEnrollment({ baseUrl: this.config.baseUrl }, sessionCsrf);
  }

  listTotpFactors(sessionCsrf: string): Promise<HostedTotpFactor[]> {
    return listHostedTotpFactors({ baseUrl: this.config.baseUrl }, sessionCsrf);
  }

  revokeTotpFactor(sessionCsrf: string, id: string): Promise<void> {
    return revokeHostedTotpFactor({ baseUrl: this.config.baseUrl }, sessionCsrf, id);
  }

  confirmTotpEnrollment(sessionCsrf: string, factorId: string, code: string): Promise<string> {
    return confirmHostedTotpEnrollment(
      { baseUrl: this.config.baseUrl },
      sessionCsrf,
      factorId,
      code,
    );
  }

  stepUpTotp(sessionCsrf: string, code: string): Promise<string> {
    return stepUpHostedTotp({ baseUrl: this.config.baseUrl }, sessionCsrf, code);
  }

  registerPasskey(
    sessionCsrf: string,
    label: string,
    options?: WebauthnCreateOptions,
  ): Promise<string> {
    return registerHostedPasskey({ baseUrl: this.config.baseUrl }, sessionCsrf, label, options);
  }

  loginPasskey(
    interaction: HostedInteraction,
    options?: WebauthnGetOptions,
  ): Promise<HostedInteraction> {
    return loginHostedPasskey({ baseUrl: this.config.baseUrl }, interaction, options);
  }

  stepUpPasskey(sessionCsrf: string, options?: WebauthnGetOptions): Promise<string> {
    return stepUpHostedPasskey({ baseUrl: this.config.baseUrl }, sessionCsrf, options);
  }

  listPasskeys(sessionCsrf: string): Promise<HostedPasskey[]> {
    return listHostedPasskeys({ baseUrl: this.config.baseUrl }, sessionCsrf);
  }

  renamePasskey(sessionCsrf: string, id: string, label: string): Promise<void> {
    return renameHostedPasskey({ baseUrl: this.config.baseUrl }, sessionCsrf, id, label);
  }

  revokePasskey(sessionCsrf: string, id: string): Promise<void> {
    return revokeHostedPasskey({ baseUrl: this.config.baseUrl }, sessionCsrf, id);
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
