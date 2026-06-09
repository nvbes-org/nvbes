import { z } from 'zod';
import { identityHttpClient } from './identity.http';

const RegionResultSchema = z.object({
  region: z.string().nullable(),
});

const SupportedRegionSchema = z.object({
  country_code: z.string(),
  data_region: z.string(),
  legal_jurisdiction: z.string(),
  primary_timezone: z.string(),
  timezones: z.array(z.string()),
  sub_region: z.string().nullable(),
  display_name: z.string().nullable(),
});

const SupportedRegionsResultSchema = z.array(SupportedRegionSchema);

const RegisterResultSchema = z.object({
  verification_resend_available_at: z.string(),
});

export type RegisterResult = z.infer<typeof RegisterResultSchema>;
export type SupportedRegion = z.infer<typeof SupportedRegionSchema>;

const LoginIdentifierResultSchema = z.object({
  next_step: z.string(),
  state_token: z.string(),
  available_methods: z.array(z.string()).nullable().optional(),
});

const LoginPasswordResultSchema = z.object({
  next_step: z.string().optional(),
  state_token: z.string().optional(),
  available_methods: z.array(z.string()).nullable().optional(),
  user: z
    .object({
      email: z.string().optional(),
      email_verified: z.boolean().optional(),
      mfa_enabled: z.boolean().optional(),
    })
    .optional(),
  session_token: z.string().optional(),
  verification_resend_available_at: z.string().nullable().optional(),
});

const WebauthnAuthStartResultSchema = z.object({
  challenge_id: z.string(),
  options: z.unknown(),
});

const LogoutResultSchema = z.undefined();

export type LoginIdentifierResult = z.infer<typeof LoginIdentifierResultSchema>;
export type LoginPasswordResult = z.infer<typeof LoginPasswordResultSchema>;
export type WebauthnAuthStartResult = z.infer<typeof WebauthnAuthStartResultSchema>;

export type RegisterInput = {
  email: string;
  firstname?: string;
  lastname?: string;
  username: string;
  birthdate?: string;
  password: string;
  workspace_name: string;
  region?: string;
};

export function detectRegion(): Promise<{ region: string | null }> {
  return identityHttpClient.get('/auth/region', RegionResultSchema);
}

export function fetchSupportedRegions(): Promise<SupportedRegion[]> {
  return identityHttpClient.get('/auth/regions', SupportedRegionsResultSchema);
}

export function submitRegister(
  input: RegisterInput,
  powNonce?: string,
  powSolution?: string,
): Promise<RegisterResult> {
  return identityHttpClient.post('/auth/register', RegisterResultSchema, {
    ...input,
    ...(powNonce ? { pow_nonce: powNonce, pow_solution: powSolution } : {}),
  });
}

export function submitLoginIdentifier(
  email: string,
  powNonce?: string,
  powSolution?: string,
  decoyLinkClicked?: boolean,
): Promise<LoginIdentifierResult> {
  return identityHttpClient.post('/auth/challenge/identifier', LoginIdentifierResultSchema, {
    email,
    ...(powNonce ? { pow_nonce: powNonce, pow_solution: powSolution } : {}),
    ...(decoyLinkClicked !== undefined ? { decoy_link_clicked: decoyLinkClicked } : {}),
  });
}

export function submitLoginPassword(
  stateToken: string,
  password: string,
): Promise<LoginPasswordResult> {
  return identityHttpClient.post('/auth/challenge/pwd', LoginPasswordResultSchema, {
    state_token: stateToken,
    password,
  });
}

export function startLoginWebAuthn(stateToken: string): Promise<WebauthnAuthStartResult> {
  return identityHttpClient.post('/auth/challenge/webauthn/start', WebauthnAuthStartResultSchema, {
    state_token: stateToken,
  });
}

export function startDiscoverableLoginWebAuthn(): Promise<WebauthnAuthStartResult> {
  return identityHttpClient.post(
    '/auth/challenge/webauthn/discoverable/start',
    WebauthnAuthStartResultSchema,
    {},
  );
}

export function finishDiscoverableLoginWebAuthn(
  challengeId: string,
  webauthnResponse: unknown,
): Promise<LoginPasswordResult> {
  return identityHttpClient.post(
    '/auth/challenge/webauthn/discoverable/finish',
    LoginPasswordResultSchema,
    {
      challenge_id: challengeId,
      webauthn_response: webauthnResponse,
    },
  );
}

export function submitLoginMfa(
  stateToken: string,
  input:
    | {
        totp_code: string;
        recovery_code?: never;
        webauthn_response?: never;
        webauthn_challenge_id?: never;
      }
    | {
        recovery_code: string;
        totp_code?: never;
        webauthn_response?: never;
        webauthn_challenge_id?: never;
      }
    | {
        webauthn_response: unknown;
        webauthn_challenge_id: string;
        totp_code?: never;
        recovery_code?: never;
      },
): Promise<LoginPasswordResult> {
  return identityHttpClient.post('/auth/challenge/mfa', LoginPasswordResultSchema, {
    state_token: stateToken,
    ...input,
  });
}

export function logoutIdentitySession(): Promise<void> {
  return identityHttpClient.post('/auth/logout', LogoutResultSchema);
}
