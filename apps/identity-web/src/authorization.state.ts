import type {
  HostedInteraction,
  HostedAuthenticationStatus,
  HostedTotpEnrollment,
} from '@nvbes/identity-sdk-web/oauth';

export interface AuthorizationState {
  stage:
    | 'loading'
    | 'login'
    | 'reauthenticate'
    | 'step-up'
    | 'security-step-up'
    | 'enrollment'
    | 'recovery-codes'
    | 'factors'
    | 'consent'
    | 'leaving'
    | 'closed';
  busy: boolean;
  interaction: HostedInteraction | null;
  authentication: HostedAuthenticationStatus | null;
  error: string | null;
  firstEnrollmentAvailable: boolean;
  totpEnrollment: HostedTotpEnrollment | null;
  recoveryCodes: string[] | null;
  hasTotp: boolean;
  managementExpiresAt: string | null;
}

/** Display hint only; never substitutes for the OAuth policy or server mutation checks. */
export function managementExpiry(state: AuthorizationState): string | null {
  const values = [state.managementExpiresAt, state.authentication?.proofExpiresAt].filter(
    (value): value is string => typeof value === 'string' && Date.parse(value) > Date.now(),
  );
  return values.sort((left, right) => Date.parse(right) - Date.parse(left))[0] ?? null;
}

/** Conservative UI freshness bound matching active factor-management policy (five minutes). */
export function managementDeadline(serverExpiry: string, requestStarted: number): string {
  const deadline = Math.min(Date.parse(serverExpiry), requestStarted + 5 * 60_000);
  if (!(deadline > Date.now())) throw new Error('Expired security proof');
  return new Date(deadline).toISOString();
}
