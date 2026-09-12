import { fetchPowChallenge, solvePowChallenge } from './pow';
import { authCredentials, handleMfaResponseError, jsonAuthHeaders } from './mfa.transport';

export type StepUpPurpose = 'password_change';

export interface StepUpCredentials {
  purpose?: StepUpPurpose;
  password?: string;
  totpCode?: string;
  webauthnResponse?: unknown;
  webauthnChallengeId?: string;
  recoveryCode?: string;
  emailCode?: string;
  emailChallengeId?: string;
}

export async function stepUp(
  baseUrl: string,
  credentials: StepUpCredentials,
  token?: string,
): Promise<{ success: boolean; valid_until: string }> {
  const powChallenge = await fetchPowChallenge(baseUrl);
  const powSolution = await solvePowChallenge(powChallenge.nonce, powChallenge.difficulty);
  const response = await fetch(`${baseUrl}/auth/step-up`, {
    method: 'POST',
    headers: jsonAuthHeaders(token),
    credentials: authCredentials(token),
    body: JSON.stringify({
      pow_nonce: powChallenge.nonce,
      pow_solution: String(powSolution),
      purpose: credentials.purpose ?? null,
      password: credentials.password ?? null,
      totp_code: credentials.totpCode ?? null,
      webauthn_response: credentials.webauthnResponse ?? null,
      webauthn_challenge_id: credentials.webauthnChallengeId ?? null,
      recovery_code: credentials.recoveryCode ?? null,
      email_code: credentials.emailCode ?? null,
      email_challenge_id: credentials.emailChallengeId ?? null,
    }),
  });
  if (!response.ok) return handleMfaResponseError(response);
  return response.json();
}

export async function requestEmailStepUpCode(
  baseUrl: string,
  purpose: StepUpPurpose,
  token?: string,
): Promise<{ challenge_id: string; expires_at: string }> {
  const response = await fetch(`${baseUrl}/auth/step-up/email/request`, {
    method: 'POST',
    headers: jsonAuthHeaders(token),
    credentials: authCredentials(token),
    body: JSON.stringify({ purpose }),
  });
  if (!response.ok) return handleMfaResponseError(response);
  return response.json();
}
