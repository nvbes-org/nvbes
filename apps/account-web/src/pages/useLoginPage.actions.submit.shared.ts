import type { FormEvent } from 'react';
import type { UseLoginPageActionsOptions } from './useLoginPage.actions.shared';

export type SubmitActionOptions = Pick<
  UseLoginPageActionsOptions,
  | 'navigate'
  | 'email'
  | 'password'
  | 'loginStateToken'
  | 'sessionToken'
  | 'mfaMethod'
  | 'totpCode'
  | 'emailCode'
  | 'recoveryCode'
  | 'decoyRef'
  | 'mutations'
  | 'resetMfaState'
  | 'setStep'
  | 'setPassword'
  | 'setIdentifierSubmitting'
  | 'setLoginStateToken'
  | 'setSessionToken'
  | 'setAvailableMethods'
  | 'setMfaMethod'
  | 'setError'
>;

export type LoginFormHandler = (event: FormEvent<HTMLFormElement>) => Promise<void>;
