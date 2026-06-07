import { createRequestHeaders } from '@nvbes/http-client';
import type {
  MfaFactorView,
  RecoveryCodesResult,
  TotpSetupResult,
  UserView,
  WorkspaceView,
} from '@nvbes/identity-sdk-core/src/types';
import {
  completeWebAuthnStepUp,
  confirmTotp,
  finishWebAuthnRegistration,
  generateRecoveryCodes,
  listMfaFactors,
  MfaError,
  registerWebAuthnCredential,
  removeMfaFactor,
  setupTotp,
  startWebAuthnAuthentication,
  startWebAuthnRegistration,
  stepUp,
} from './mfa';
import { generateCodeChallenge, generateCodeVerifier, type PKCEChallenge } from './pkce';
import { memoryStorage, type WebStorage } from './storage';
import {
  createWebAuthnCredential,
  getConditionalWebAuthnCredential,
  getWebAuthnCredential,
  getWebAuthnSupport,
  isConditionalMediationSupported,
  normalizeWebAuthnError,
  parseRequestOptions,
  serializeCredential,
  WebauthnBrowserError,
  type WebauthnRegistrationKind,
} from './webauthn';

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

  /**
   * Génère un défi PKCE (recommandé pour web)
   */
  async createPKCEChallenge(): Promise<PKCEChallenge> {
    const codeVerifier = generateCodeVerifier();
    const codeChallenge = await generateCodeChallenge(codeVerifier);

    this.storage.saveCodeVerifier(codeVerifier);

    return { codeVerifier, codeChallenge };
  }

  /**
   * Génère l'URL d'autorisation OAuth2
   */
  getAuthorizationUrl(
    options: { scope?: string; state?: string; codeChallenge?: string } = {},
  ): string {
    const params = new URLSearchParams({
      response_type: 'code',
      client_id: this.config.clientId,
      redirect_uri: this.config.redirectUri,
      scope: options.scope ?? 'openid profile email',
      ...(options.state && { state: options.state }),
      ...(options.codeChallenge && {
        code_challenge: options.codeChallenge,
        code_challenge_method: 'S256',
      }),
    });

    return `${this.config.baseUrl}/oauth/authorize?${params.toString()}`;
  }

  /**
   * Redirige vers la page de login (web - utilise cookies)
   */
  async redirectToLogin(
    options: { scope?: string; state?: string; usePKCE?: boolean } = {},
  ): Promise<void> {
    let codeChallenge: string | undefined;

    if (options.usePKCE ?? true) {
      const pkce = await this.createPKCEChallenge();
      codeChallenge = pkce.codeChallenge;
    }

    window.location.href = this.getAuthorizationUrl({
      scope: options.scope,
      state: options.state,
      codeChallenge,
    });
  }

  /**
   * Récupère les infos utilisateur via cookie de session
   */
  async getCurrentUser(): Promise<UserView> {
    const response = await fetch(`${this.config.baseUrl}/auth/me`, {
      credentials: 'include',
    });

    if (!response.ok) {
      throw new Error(`Failed to fetch user: ${response.statusText}`);
    }

    return response.json();
  }

  /**
   * Déconnexion - appel au backend pour supprimer les cookies
   */
  async logout(): Promise<void> {
    const headers: Record<string, string> = {};
    if (typeof document !== 'undefined') {
      const match = document.cookie.match(/(?:^|;\s*)csrf_token=([^;]*)/);
      const csrfToken = match?.[1];
      if (csrfToken) {
        headers['X-CSRF-Token'] = csrfToken;
      }
    }

    await fetch(`${this.config.baseUrl}/auth/logout`, {
      method: 'POST',
      credentials: 'include',
      headers: createRequestHeaders('POST', headers),
    });
  }

  /**
   * Vérifie si l'utilisateur a un cookie de session valide
   */
  async isAuthenticated(): Promise<boolean> {
    try {
      await this.getCurrentUser();
      return true;
    } catch {
      return false;
    }
  }

  // ==================== MFA Methods ====================

  /**
   * Liste les facteurs MFA de l'utilisateur.
   * Quand `token` est absent, utilise le cookie de session.
   */
  async listMfaFactors(
    token?: string,
  ): Promise<{ factors: MfaFactorView[]; mfa_enabled: boolean }> {
    return listMfaFactors(this.config.baseUrl, token);
  }

  /**
   * Configure TOTP (retourne secret + URI pour QR code).
   * Le backend requiert un step-up préalable.
   */
  async setupTotp(label?: string, token?: string): Promise<TotpSetupResult> {
    return setupTotp(this.config.baseUrl, label, token);
  }

  /**
   * Confirme le code TOTP.
   */
  async confirmTotp(
    factorId: string,
    code: string,
    token?: string,
  ): Promise<{ factor: MfaFactorView; mfa_enabled: boolean }> {
    return confirmTotp(this.config.baseUrl, factorId, code, token);
  }

  /**
   * Démarre l'enregistrement WebAuthn.
   */
  async startWebAuthnRegistration(
    label?: string,
    kind: WebauthnRegistrationKind = 'passkey',
    token?: string,
  ): Promise<{
    factorId: string;
    options: PublicKeyCredentialCreationOptions;
  }> {
    return startWebAuthnRegistration(this.config.baseUrl, label, kind, token);
  }

  /**
   * Termine l'enregistrement WebAuthn.
   */
  async finishWebAuthnRegistration(
    factorId: string,
    credential: PublicKeyCredential,
    token?: string,
  ): Promise<void> {
    return finishWebAuthnRegistration(this.config.baseUrl, factorId, credential, token);
  }

  /**
   * Enregistre un facteur WebAuthn de bout en bout avec pré-détection et timeout.
   */
  async registerWebAuthnCredential(
    label?: string,
    kind: WebauthnRegistrationKind = 'passkey',
    token?: string,
  ): Promise<void> {
    return registerWebAuthnCredential(this.config.baseUrl, label, kind, token);
  }

  /**
   * Démarre l'authentification WebAuthn (pour step-up ou login MFA).
   */
  async startWebAuthnAuthentication(token?: string): Promise<{
    challengeId: string;
    options: PublicKeyCredentialRequestOptions;
  }> {
    return startWebAuthnAuthentication(this.config.baseUrl, token);
  }

  /**
   * Effectue un step-up WebAuthn complet avec pré-détection et timeout.
   */
  async completeWebAuthnStepUp(token?: string): Promise<void> {
    return completeWebAuthnStepUp(this.config.baseUrl, token);
  }

  /**
   * Génère de nouveaux codes de récupération.
   * Le backend requiert un step-up préalable.
   */
  async generateRecoveryCodes(password: string, token?: string): Promise<RecoveryCodesResult> {
    return generateRecoveryCodes(this.config.baseUrl, password, token);
  }

  /**
   * Supprime un facteur MFA.
   * Le backend requiert un step-up préalable.
   */
  async removeMfaFactor(factorId: string, token?: string): Promise<void> {
    return removeMfaFactor(this.config.baseUrl, factorId, token);
  }

  /**
   * Step-up : re-vérifie l'identité avant une opération sensible.
   */
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
export type { DecoyField, DecoyLinkTracker } from './bot-guard.decoy';
export { createDecoyField, createDecoyLinks, mountDecoyField } from './bot-guard.decoy';
export type {
  AutomationSignals,
  BehavioralSignals,
  BotSignals,
  EnvironmentSignals,
  ExtensionSignals,
  FingerprintSignals,
  SignalsCollector,
} from './bot-guard.signals';
// --- Bot Guard Signals & Decoy ---
export {
  attachBehavioralObserver,
  collectAutomationSignals,
  collectEnvironmentSignals,
  collectExtensionSignals,
  collectFingerprintSignals,
  createSignalsCollector,
} from './bot-guard.signals';
export {
  getStoredPasswordCredential,
  isCredentialManagementSupported,
  preventAutoSignIn,
  storePasswordCredential,
} from './credential-management';
export {
  configureDpopCryptoWorker,
  createDpopProof,
  dpopFetch,
  ensureDpopKeyPair,
  extractNonceFromResponse,
  generateDpopKeyPair,
  getCachedJkt,
  getCachedKeyPair,
  getCachedPublicJwk,
  getDpopNonce,
  isDpopSupported,
  setDpopNonce,
} from './dpop';
export type { PowChallenge } from './pow';
export { fetchPowChallenge, solvePowChallenge } from './pow';
export type {
  WebauthnCreateOptions,
  WebauthnRegistrationKind,
  WebauthnSupportReport,
  WebauthnUnsupportedReason,
} from './webauthn';
export type { PKCEChallenge, WebStorage };
export {
  completeWebAuthnStepUp,
  confirmTotp,
  createWebAuthnCredential,
  finishWebAuthnRegistration,
  generateCodeChallenge,
  generateCodeVerifier,
  generateRecoveryCodes,
  getConditionalWebAuthnCredential,
  getWebAuthnCredential,
  getWebAuthnSupport,
  isConditionalMediationSupported,
  listMfaFactors,
  MfaError,
  normalizeWebAuthnError,
  parseRequestOptions,
  registerWebAuthnCredential,
  removeMfaFactor,
  serializeCredential,
  setupTotp,
  startWebAuthnAuthentication,
  startWebAuthnRegistration,
  stepUp,
  WebauthnBrowserError,
};
