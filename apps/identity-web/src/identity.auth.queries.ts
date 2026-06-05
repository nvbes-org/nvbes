import { effectMutationFn, effectQueryFn } from '@nvbes/web-runtime';
import {
  detectRegionWorkflow,
  fetchSupportedRegionsWorkflow,
  logoutIdentitySessionWorkflow,
  type RegisterInput,
  startLoginWebAuthnWorkflow,
  submitLoginIdentifierWorkflow,
  submitLoginMfaWorkflow,
  submitLoginPasswordWorkflow,
  submitRegisterWorkflow,
} from './identity.auth.workflow';

export const identityAuthMutationKeys = {
  loginIdentifier: ['identity', 'auth', 'login-identifier'] as const,
  loginPassword: ['identity', 'auth', 'login-password'] as const,
  loginMfa: ['identity', 'auth', 'login-mfa'] as const,
  loginWebauthnStart: ['identity', 'auth', 'login-webauthn-start'] as const,
  logout: ['identity', 'auth', 'logout'] as const,
  register: ['identity', 'auth', 'register'] as const,
  regionDetect: ['identity', 'auth', 'region-detect'] as const,
  supportedRegions: ['identity', 'auth', 'supported-regions'] as const,
};

export const submitRegisterMutationFn = effectMutationFn(
  (input: { data: RegisterInput; powNonce?: string; powSolution?: string }) =>
    submitRegisterWorkflow(input.data, input.powNonce, input.powSolution),
);

export const detectRegionQueryFn = effectQueryFn(() => detectRegionWorkflow());
export const supportedRegionsQueryFn = effectQueryFn(() => fetchSupportedRegionsWorkflow());

export const submitLoginIdentifierMutationFn = effectMutationFn(
  (vars: {
    email: string;
    powNonce?: string;
    powSolution?: string;
    decoy_link_clicked?: boolean;
  }) =>
    submitLoginIdentifierWorkflow(
      vars.email,
      vars.powNonce,
      vars.powSolution,
      vars.decoy_link_clicked,
    ),
);

export const submitLoginPasswordMutationFn = effectMutationFn(
  (variables: { stateToken: string; password: string }) =>
    submitLoginPasswordWorkflow(variables.stateToken, variables.password),
);

export const startLoginWebAuthnMutationFn = effectMutationFn((stateToken: string) =>
  startLoginWebAuthnWorkflow(stateToken),
);

export const submitLoginMfaMutationFn = effectMutationFn(
  (
    variables:
      | { stateToken: string; totpCode: string }
      | { stateToken: string; recoveryCode: string }
      | {
          stateToken: string;
          webauthnResponse: unknown;
          webauthnChallengeId: string;
        },
  ) => {
    if ('totpCode' in variables) {
      return submitLoginMfaWorkflow(variables.stateToken, {
        totp_code: variables.totpCode,
      });
    }

    if ('recoveryCode' in variables) {
      return submitLoginMfaWorkflow(variables.stateToken, {
        recovery_code: variables.recoveryCode,
      });
    }

    return submitLoginMfaWorkflow(variables.stateToken, {
      webauthn_response: variables.webauthnResponse,
      webauthn_challenge_id: variables.webauthnChallengeId,
    });
  },
);

export const logoutIdentitySessionMutationFn = effectMutationFn(() =>
  logoutIdentitySessionWorkflow(),
);
