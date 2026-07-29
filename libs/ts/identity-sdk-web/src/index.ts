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
  MfaError,
  registerWebAuthnCredential,
  requestEmailStepUpCode,
  removeMfaFactor,
  setupTotp,
  startWebAuthnAuthentication,
  startWebAuthnRegistration,
  stepUp,
  type StepUpPurpose,
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
  getAuthorizationUrl(options: {
    scope?: string;
    state: string;
    codeChallenge: string;
    nonce: string;
  }): string {
    if (!options.state.trim()) {
      throw new Error('OAuth state is required.');
    }
    if (!options.codeChallenge.trim()) {
      throw new Error('PKCE code challenge is required.');
    }
    if (!options.nonce.trim()) {
      throw new Error('OIDC nonce is required.');
    }

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

  /**
   * Redirige vers la page de login (web - utilise cookies)
   */
  async redirectToLogin(options: { scope?: string; state: string; nonce: string }): Promise<void> {
    const pkce = await this.createPKCEChallenge();

    window.location.href = this.getAuthorizationUrl({
      scope: options.scope,
      state: options.state,
      codeChallenge: pkce.codeChallenge,
      nonce: options.nonce,
    });
  }

  /**
   * Récupère les infos utilisateur via cookie de session
   */
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

  /**
   * Déconnexion - appel au backend pour supprimer les cookies
   */
  async logout(): Promise<void> {
    const headers: Record<string, string> = {};
    const authuser = readCurrentAuthuser();
    if (authuser) {
      headers['X-Auth-User'] = authuser;
    }
    const csrfToken = readScopedCsrfToken(authuser);
    if (csrfToken) {
      headers['X-CSRF-Token'] = csrfToken;
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
    options: { limit?: number; cursor?: string } = {},
  ): Promise<{
    factors: MfaFactorView[];
    mfa_enabled: boolean;
    next_cursor: string | null;
    has_more: boolean;
  }> {
    return listMfaFactors(this.config.baseUrl, token, options);
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
  async generateRecoveryCodes(password?: string, token?: string): Promise<RecoveryCodesResult> {
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
export { collectBotIntegritySignals } from './bot-integrity';
export type { BotIntegritySignals } from './bot-integrity';
export { collectDeviceProfile } from './device-profile';
export type {
  DeviceFormFactor,
  DevicePlatform,
  DeviceProfile,
  ScreenBucket,
} from './device-profile';
export {
  detectBrowser,
  detectBrowserVersion,
  detectDevice,
  detectDeviceType,
  detectOS,
  parseUserAgent,
} from './user-agent';
export type {
  BrowserDetectionOptions,
  BrowserHints,
  BrowserName,
  DeviceName,
  DeviceTypeOptions,
  OperatingSystem,
  OperatingSystemName,
  UserAgentDetectionOptions,
  UserAgentDeviceType,
  UserAgentInfo,
} from './user-agent';
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
export { DeviceMonitor } from './device-monitor';
export type { BatteryStatus, DevicePerformanceState } from './device-monitor';
export type { PowChallenge, PowSolverOptions, PowSolverProgress } from './pow';
export { fetchPowChallenge, solvePowChallenge } from './pow';
export type {
  WebauthnCreateOptions,
  WebauthnRegistrationKind,
  WebauthnSupportReport,
  WebauthnUnsupportedReason,
} from './webauthn';
export type { PKCEChallenge, StepUpPurpose, WebStorage };
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
  readCurrentAuthuser,
  readScopedCsrfToken,
  registerWebAuthnCredential,
  removeMfaFactor,
  requestEmailStepUpCode,
  serializeCredential,
  setupTotp,
  startWebAuthnAuthentication,
  startWebAuthnRegistration,
  stepUp,
  WebauthnBrowserError,
};
