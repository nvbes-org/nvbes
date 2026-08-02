import { normalizeClientError } from '@nvbes/web-runtime';
import {
  fetchRegistrationAvailability,
  type LoginIdentifierResult,
  type LoginPasswordResult,
  logoutIdentitySession,
  type RegisterInput,
  type RegisterResult,
  type RegistrationAvailabilityResult,
  startLoginWebAuthn,
  submitLoginIdentifier,
  submitLoginMfa,
  submitLoginPassword,
  submitRegister,
  type WebauthnAuthStartResult,
} from './identity.auth.api';

export type { RegisterInput };

export async function registerIdentityAccount(input: {
  data: RegisterInput;
  powNonce: string;
  powSolution: string;
}): Promise<RegisterResult> {
  try {
    return await submitRegister(input.data, input.powNonce, input.powSolution);
  } catch (error) {
    throw normalizeClientError(error);
  }
}

export async function checkRegistrationIdentifierAvailability(input: {
  email: string;
  signal: AbortSignal;
}): Promise<RegistrationAvailabilityResult> {
  try {
    return await fetchRegistrationAvailability(input.email, input.signal);
  } catch (error) {
    throw normalizeClientError(error);
  }
}

export async function submitLoginIdentifierStep(vars: {
  email: string;
  powNonce: string;
  powSolution: string;
  decoy_link_clicked?: boolean;
  device_fingerprint?: import('@nvbes/identity-sdk-web').DeviceProfile;
  bot_signals?: import('@nvbes/identity-sdk-web').BotIntegritySignals;
}): Promise<LoginIdentifierResult> {
  try {
    return await submitLoginIdentifier(
      vars.email,
      vars.powNonce,
      vars.powSolution,
      vars.decoy_link_clicked,
      vars.device_fingerprint,
      vars.bot_signals,
    );
  } catch (error) {
    throw normalizeClientError(error);
  }
}

export async function submitLoginPasswordStep(variables: {
  stateToken: string;
  password: string;
}): Promise<LoginPasswordResult> {
  try {
    return await submitLoginPassword(variables.stateToken, variables.password);
  } catch (error) {
    throw normalizeClientError(error);
  }
}

export async function startLoginWebauthnStep(stateToken: string): Promise<WebauthnAuthStartResult> {
  try {
    return await startLoginWebAuthn(stateToken);
  } catch (error) {
    throw normalizeClientError(error);
  }
}

export type LoginMfaStepInput =
  | { stateToken: string; totpCode: string }
  | { stateToken: string; recoveryCode: string }
  | {
      stateToken: string;
      webauthnResponse: unknown;
      webauthnChallengeId: string;
    };

export async function submitLoginMfaStep(
  variables: LoginMfaStepInput,
): Promise<LoginPasswordResult> {
  try {
    if ('totpCode' in variables) {
      return await submitLoginMfa(variables.stateToken, {
        totp_code: variables.totpCode,
      });
    }

    if ('recoveryCode' in variables) {
      return await submitLoginMfa(variables.stateToken, {
        recovery_code: variables.recoveryCode,
      });
    }

    return await submitLoginMfa(variables.stateToken, {
      webauthn_response: variables.webauthnResponse,
      webauthn_challenge_id: variables.webauthnChallengeId,
    });
  } catch (error) {
    throw normalizeClientError(error);
  }
}

export async function logoutIdentitySessionFn(): Promise<void> {
  try {
    await logoutIdentitySession();
  } catch (error) {
    throw normalizeClientError(error);
  }
}
