import {
  checkRegistrationIdentifierAvailability,
  logoutIdentitySessionFn,
  registerIdentityAccount,
  startLoginWebauthnStep,
  submitLoginIdentifierStep,
  submitLoginMfaStep,
  submitLoginPasswordStep,
} from './identity.auth.functions';

export const identityAuthQueryKeys = {
  registrationAvailability: (email: string) =>
    ['identity', 'auth', 'registration-availability', email] as const,
};

export const identityAuthMutationKeys = {
  loginIdentifier: ['identity', 'auth', 'login-identifier'] as const,
  loginPassword: ['identity', 'auth', 'login-password'] as const,
  loginMfa: ['identity', 'auth', 'login-mfa'] as const,
  loginWebauthnStart: ['identity', 'auth', 'login-webauthn-start'] as const,
  logout: ['identity', 'auth', 'logout'] as const,
  register: ['identity', 'auth', 'register'] as const,
};

export const submitRegisterMutationFn = registerIdentityAccount;

export const registrationAvailabilityQueryFn = checkRegistrationIdentifierAvailability;

export const submitLoginIdentifierMutationFn = submitLoginIdentifierStep;

export const submitLoginPasswordMutationFn = submitLoginPasswordStep;

export const startLoginWebAuthnMutationFn = startLoginWebauthnStep;

export const submitLoginMfaMutationFn = submitLoginMfaStep;

export const logoutIdentitySessionMutationFn = logoutIdentitySessionFn;
