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
  firstname: string;
  lastname: string;
  username: string;
  birthdate?: string;
  password: string;
  workspace_name: string;
  region?: string;
  legal_documents_accepted: boolean;
  marketing_emails_accepted: boolean;
};

const AUTH_REQUEST_TIMEOUT_MS = 20_000;

class AuthRequestTimeoutError extends Error {
  constructor(label: string) {
    super(`${label} n'a pas répondu à temps. Vérifiez que l'API Identity est bien disponible.`);
    this.name = 'AuthRequestTimeoutError';
  }
}

async function withAuthRequestTimeout<T>(
  label: string,
  request: (signal: AbortSignal) => Promise<T>,
): Promise<T> {
  const controller = new AbortController();
  const timeoutId = globalThis.setTimeout(() => {
    controller.abort(new AuthRequestTimeoutError(label));
  }, AUTH_REQUEST_TIMEOUT_MS);

  try {
    return await request(controller.signal);
  } catch (error) {
    if (controller.signal.aborted) {
      const reason = controller.signal.reason;
      throw reason instanceof Error ? reason : new AuthRequestTimeoutError(label);
    }
    throw error;
  } finally {
    globalThis.clearTimeout(timeoutId);
  }
}

export function detectRegion(): Promise<{ region: string | null }> {
  return identityHttpClient.get('/auth/region', RegionResultSchema);
}

export function fetchSupportedRegions(): Promise<SupportedRegion[]> {
  return identityHttpClient.get('/auth/regions', SupportedRegionsResultSchema);
}

export function submitRegister(
  input: RegisterInput,
  powNonce: string,
  powSolution: string,
): Promise<RegisterResult> {
  return withAuthRequestTimeout("L'inscription", (signal) =>
    identityHttpClient.post(
      '/auth/register',
      RegisterResultSchema,
      {
        ...input,
        pow_nonce: powNonce,
        pow_solution: powSolution,
      },
      { signal },
    ),
  );
}

export function submitLoginIdentifier(
  email: string,
  powNonce: string,
  powSolution: string,
  decoyLinkClicked?: boolean,
): Promise<LoginIdentifierResult> {
  return withAuthRequestTimeout("L'identification", (signal) =>
    identityHttpClient.post(
      '/auth/challenge/identifier',
      LoginIdentifierResultSchema,
      {
        email,
        pow_nonce: powNonce,
        pow_solution: powSolution,
        ...(decoyLinkClicked !== undefined ? { decoy_link_clicked: decoyLinkClicked } : {}),
      },
      { signal },
    ),
  );
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
        email_code?: never;
        recovery_code?: never;
        webauthn_response?: never;
        webauthn_challenge_id?: never;
      }
    | {
        email_code: string;
        totp_code?: never;
        recovery_code?: never;
        webauthn_response?: never;
        webauthn_challenge_id?: never;
      }
    | {
        recovery_code: string;
        totp_code?: never;
        email_code?: never;
        webauthn_response?: never;
        webauthn_challenge_id?: never;
      }
    | {
        webauthn_response: unknown;
        webauthn_challenge_id: string;
        totp_code?: never;
        email_code?: never;
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
