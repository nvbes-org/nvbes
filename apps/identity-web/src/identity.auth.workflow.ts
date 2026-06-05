import { Effect } from 'effect';
import {
  detectRegion,
  fetchSupportedRegions,
  logoutIdentitySession,
  type RegisterInput,
  startLoginWebAuthn,
  submitLoginIdentifier,
  submitLoginMfa,
  submitLoginPassword,
  submitRegister,
  type SupportedRegion,
} from './identity.auth.api';

export type { RegisterInput };

function detectRegionFromBrowser(supportedRegions: SupportedRegion[]): {
  region: string | null;
  reliability: 'medium' | 'low' | 'none';
} {
  try {
    const userTimezone = Intl.DateTimeFormat().resolvedOptions().timeZone;
    if (userTimezone) {
      const match = supportedRegions.find(
        (r) => r.timezones.includes(userTimezone) || r.primary_timezone === userTimezone,
      );
      if (match) {
        return { region: match.country_code, reliability: 'medium' };
      }
    }
  } catch {
    // Ignore Intl errors
  }

  try {
    const locale = navigator.language || (navigator.languages && navigator.languages[0]);
    if (locale) {
      const parts = locale.split('-');
      const countryCode = parts.length > 1 ? parts[1].toUpperCase() : parts[0].toUpperCase();
      const match = supportedRegions.find((r) => r.country_code === countryCode);
      if (match) {
        return { region: match.country_code, reliability: 'low' };
      }
    }
  } catch {
    // Ignore navigator errors
  }

  return { region: null, reliability: 'none' };
}

export function detectRegionWorkflow() {
  return Effect.gen(function* () {
    const serverResult = yield* Effect.tryPromise({
      try: () => detectRegion(),
      catch: (error) => error,
    }) as any;

    if (
      serverResult &&
      typeof serverResult === 'object' &&
      'region' in serverResult &&
      serverResult.region
    ) {
      return { region: serverResult.region as string, reliability: 'high' as const };
    }

    const supportedRegions = yield* Effect.tryPromise({
      try: () => fetchSupportedRegions(),
      catch: (error) => error,
    }) as any;

    if (Array.isArray(supportedRegions)) {
      return detectRegionFromBrowser(supportedRegions);
    }

    return { region: null, reliability: 'none' as const };
  }) as Effect.Effect<any, any, never>;
}

export function fetchSupportedRegionsWorkflow() {
  return Effect.tryPromise({
    try: () => fetchSupportedRegions(),
    catch: (error) => error,
  });
}

export function submitRegisterWorkflow(
  input: RegisterInput,
  powNonce?: string,
  powSolution?: string,
) {
  return Effect.tryPromise({
    try: () => submitRegister(input, powNonce, powSolution),
    catch: (error) => error,
  });
}

export function submitLoginIdentifierWorkflow(
  email: string,
  powNonce?: string,
  powSolution?: string,
  decoyLinkClicked?: boolean,
) {
  return Effect.tryPromise({
    try: () => submitLoginIdentifier(email, powNonce, powSolution, decoyLinkClicked),
    catch: (error) => error,
  });
}

export function submitLoginPasswordWorkflow(stateToken: string, password: string) {
  return Effect.tryPromise({
    try: () => submitLoginPassword(stateToken, password),
    catch: (error) => error,
  });
}

export function startLoginWebAuthnWorkflow(stateToken: string) {
  return Effect.tryPromise({
    try: () => startLoginWebAuthn(stateToken),
    catch: (error) => error,
  });
}

export function submitLoginMfaWorkflow(
  stateToken: string,
  input: Parameters<typeof submitLoginMfa>[1],
) {
  return Effect.tryPromise({
    try: () => submitLoginMfa(stateToken, input),
    catch: (error) => error,
  });
}

export function logoutIdentitySessionWorkflow() {
  return Effect.tryPromise({
    try: () => logoutIdentitySession(),
    catch: (error) => error,
  });
}
