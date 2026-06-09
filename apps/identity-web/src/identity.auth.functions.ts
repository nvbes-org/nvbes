import { normalizeClientError } from '@nvbes/web-runtime';
import {
  detectRegion,
  fetchSupportedRegions,
  type LoginIdentifierResult,
  type LoginPasswordResult,
  logoutIdentitySession,
  type RegisterInput,
  type RegisterResult,
  startLoginWebAuthn,
  submitLoginIdentifier,
  submitLoginMfa,
  submitLoginPassword,
  submitRegister,
  type SupportedRegion,
  type WebauthnAuthStartResult,
} from './identity.auth.api';

export type { RegisterInput };

type RegionDetection = {
  region: string | null;
  reliability: 'high' | 'medium' | 'low' | 'none';
};

function detectRegionFromBrowser(supportedRegions: SupportedRegion[]): {
  region: string | null;
  reliability: 'medium' | 'low' | 'none';
} {
  try {
    const userTimezone = Intl.DateTimeFormat().resolvedOptions().timeZone;
    if (userTimezone) {
      const match = supportedRegions.find(
        (region) =>
          region.timezones.includes(userTimezone) || region.primary_timezone === userTimezone,
      );
      if (match) {
        return { region: match.country_code, reliability: 'medium' };
      }
    }
  } catch {
    // Browser region detection is best-effort.
  }

  try {
    const locale = navigator.language || (navigator.languages && navigator.languages[0]);
    if (locale) {
      const parts = locale.split('-');
      const countryCode = parts.length > 1 ? parts[1].toUpperCase() : parts[0].toUpperCase();
      const match = supportedRegions.find((region) => region.country_code === countryCode);
      if (match) {
        return { region: match.country_code, reliability: 'low' };
      }
    }
  } catch {
    // Browser region detection is best-effort.
  }

  return { region: null, reliability: 'none' };
}

export async function detectRegistrationRegion(): Promise<RegionDetection> {
  try {
    const serverResult = await detectRegion();

    if (serverResult.region) {
      return { region: serverResult.region, reliability: 'high' };
    }

    return detectRegionFromBrowser(await fetchSupportedRegions());
  } catch (error) {
    throw normalizeClientError(error);
  }
}

export async function loadSupportedRegions(): Promise<SupportedRegion[]> {
  try {
    return await fetchSupportedRegions();
  } catch (error) {
    throw normalizeClientError(error);
  }
}

export async function registerIdentityAccount(input: {
  data: RegisterInput;
  powNonce?: string;
  powSolution?: string;
}): Promise<RegisterResult> {
  try {
    return await submitRegister(input.data, input.powNonce, input.powSolution);
  } catch (error) {
    throw normalizeClientError(error);
  }
}

export async function submitLoginIdentifierStep(vars: {
  email: string;
  powNonce?: string;
  powSolution?: string;
  decoy_link_clicked?: boolean;
}): Promise<LoginIdentifierResult> {
  try {
    return await submitLoginIdentifier(
      vars.email,
      vars.powNonce,
      vars.powSolution,
      vars.decoy_link_clicked,
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
