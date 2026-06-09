import { type AccountEntry } from '@nvbes/identity-client';
import { type UseNavigateResult } from '@tanstack/react-router';
import { type MutableRefObject } from 'react';

import { type OAuthAuthorizeRequest } from '../identity.oauth';
import { type MfaMethod } from './LoginPage.mfa';
import type {
  IdentifierMutateAsync,
  MfaMutateAsync,
  PasswordMutateAsync,
  WebauthnStartMutateAsync,
} from './useLoginPage.steps.shared';

export type LoginMutations = {
  loginIdentifierMutation: { mutateAsync: IdentifierMutateAsync };
  loginPasswordMutation: { mutateAsync: PasswordMutateAsync };
  loginMfaMutation: { mutateAsync: MfaMutateAsync };
  loginWebauthnStartMutation: { mutateAsync: WebauthnStartMutateAsync };
};

export type NavigateFn = UseNavigateResult<string>;

export type UseLoginPageActionsOptions = {
  navigate: NavigateFn;
  oauthRequest: OAuthAuthorizeRequest | null;
  connectedAccounts: AccountEntry[];
  setConnectedAccounts: (
    value: AccountEntry[] | ((prev: AccountEntry[]) => AccountEntry[]),
  ) => void;
  email: string;
  password: string;
  loginStateToken: string | null;
  sessionToken: string | null;
  mfaMethod: MfaMethod | null;
  totpCode: string;
  recoveryCode: string;
  decoyRef: MutableRefObject<{ wasClicked(): boolean } | null>;
  mutations: LoginMutations;
  resetMfaState: () => void;
  setCheckingAuth: (value: boolean) => void;
  setStep: (value: import('./LoginProgress').LoginStep) => void;
  setEmail: (value: string) => void;
  setPassword: (value: string) => void;
  setIdentifierSubmitting: (value: boolean) => void;
  setLoginStateToken: (value: string | null) => void;
  setSessionToken: (value: string | null) => void;
  setAvailableMethods: (value: MfaMethod[]) => void;
  setMfaMethod: (value: MfaMethod | null) => void;
  setError: (value: string | null) => void;
  navigateToAccount: () => void;
};
