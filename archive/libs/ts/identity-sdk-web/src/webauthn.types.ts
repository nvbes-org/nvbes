export type WebauthnRegistrationKind = 'passkey' | 'security_key';

export type WebauthnUnsupportedReason =
  | 'missing_window'
  | 'missing_public_key_credential'
  | 'missing_credentials_api'
  | 'insecure_context'
  | 'embedded_runtime';

export interface WebauthnSupportReport {
  supported: boolean;
  has_public_key_credential: boolean;
  has_credentials_api: boolean;
  is_secure_context: boolean;
  is_iframe: boolean;
  is_likely_embedded_runtime: boolean;
  platform_authenticator_available: boolean | null;
  conditional_mediation_available: boolean | null;
  reasons: WebauthnUnsupportedReason[];
}

export interface WebauthnCreateOptions {
  timeoutMs?: number;
  signal?: AbortSignal;
}

export interface WebauthnGetOptions extends WebauthnCreateOptions {
  mediation?: CredentialMediationRequirement;
}

export class WebauthnBrowserError extends Error {
  constructor(
    public readonly code: string,
    message: string,
    public readonly cause?: unknown,
  ) {
    super(message);
    this.name = 'WebauthnBrowserError';
  }
}
