import type { UseMutateAsyncFunction } from '@tanstack/react-query';

import {
  startLoginWebAuthnMutationFn,
  submitLoginIdentifierMutationFn,
  submitLoginMfaMutationFn,
  submitLoginPasswordMutationFn,
} from '../identity.auth.queries';

export type ResetMfaState = () => void;
export type IdentifierStepInput = Parameters<typeof submitLoginIdentifierMutationFn>[0];
export type IdentifierStepResult = Awaited<ReturnType<typeof submitLoginIdentifierMutationFn>>;
export type PasswordStepInput = Parameters<typeof submitLoginPasswordMutationFn>[0];
export type PasswordStepResult = Awaited<ReturnType<typeof submitLoginPasswordMutationFn>>;
export type MfaStepInput = Parameters<typeof submitLoginMfaMutationFn>[0];
export type MfaStepResult = Awaited<ReturnType<typeof submitLoginMfaMutationFn>>;
export type WebauthnStartResult = Awaited<ReturnType<typeof startLoginWebAuthnMutationFn>>;

export type IdentifierMutateAsync = UseMutateAsyncFunction<
  IdentifierStepResult,
  Error,
  IdentifierStepInput,
  unknown
>;

export type PasswordMutateAsync = UseMutateAsyncFunction<
  PasswordStepResult,
  Error,
  PasswordStepInput,
  unknown
>;

export type MfaMutateAsync = UseMutateAsyncFunction<MfaStepResult, Error, MfaStepInput, unknown>;

export type WebauthnStartMutateAsync = UseMutateAsyncFunction<
  WebauthnStartResult,
  Error,
  string,
  unknown
>;
