/**
 * Types communs pour nvbes Identity SDK
 */

export interface AuthConfig {
  baseUrl: string;
  clientId: string;
  clientSecret?: string;
  redirectUri: string;
}

export interface UserView {
  id: string;
  email: string;
  display_name: string;
  firstname?: string;
  lastname?: string;
  username?: string;
  birthdate?: string;
  region?: string;
  email_verified: boolean;
  mfa_enabled: boolean;
  created_at: string;
}

export interface WorkspaceView {
  id: string;
  name: string;
  slug: string;
  role: string;
}

export interface MfaFactorView {
  id: string;
  factor_type: string;
  kind?: 'passkey' | 'security_key' | string;
  status: string;
  label?: string;
  created_at: string;
  confirmed_at?: string;
  last_used_at?: string;
}

export interface MfaFactorsResult {
  factors: MfaFactorView[];
  mfa_enabled: boolean;
}

export interface TotpSetupResult {
  factor: MfaFactorView;
  secret_base32: string;
  provisioning_uri: string;
}

export interface TotpConfirmResult {
  factor: MfaFactorView;
  mfa_enabled: boolean;
}

export interface WebauthnCreationOptionsJSON {
  challenge: string;
  user: {
    id: string;
    name: string;
    displayName: string;
  };
  pubKeyCredParams: Array<{
    type: string;
    alg: number;
  }>;
  excludeCredentials?: Array<{
    id: string;
    type: string;
    transports?: string[];
  }>;
  authenticatorSelection?: {
    authenticatorAttachment?: string;
    requireResidentKey?: boolean;
    residentKey?: string;
    userVerification?: string;
  };
  timeout?: number;
  attestation?: string;
  rp: {
    id: string;
    name: string;
  };
}

export interface WebauthnRegisterStartResult {
  factor_id: string;
  options: WebauthnCreationOptionsJSON | { publicKey: WebauthnCreationOptionsJSON };
}

export interface RecoveryCodesResult {
  codes: string[];
}

export interface WebauthnRequestOptionsJSON {
  challenge: string;
  allowCredentials?: Array<{
    id: string;
    type: string;
    transports?: string[];
  }>;
  timeout?: number;
  rpId?: string;
  userVerification?: string;
}

export interface WebauthnAuthStartResult {
  challenge_id: string;
  options: WebauthnRequestOptionsJSON | { publicKey: WebauthnRequestOptionsJSON };
}

export interface WebauthnCredentialJSON {
  id: string;
  rawId: string;
  type: string;
  response: {
    clientDataJSON: string;
    attestationObject?: string;
    authenticatorData?: string;
    signature?: string;
    userHandle?: string;
  };
}

export interface TokenResponse {
  access_token: string;
  refresh_token?: string;
  token_type: string;
  expires_in: number;
  scope: string;
}
