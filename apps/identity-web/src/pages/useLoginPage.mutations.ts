import { useMutation } from '@tanstack/react-query';

import {
  identityAuthMutationKeys,
  startLoginWebAuthnMutationFn,
  submitLoginIdentifierMutationFn,
  submitLoginMfaMutationFn,
  submitLoginPasswordMutationFn,
} from '../identity.auth.queries';

export function useLoginPageMutations() {
  const loginIdentifierMutation = useMutation({
    mutationKey: identityAuthMutationKeys.loginIdentifier,
    mutationFn: submitLoginIdentifierMutationFn,
  });
  const loginPasswordMutation = useMutation({
    mutationKey: identityAuthMutationKeys.loginPassword,
    mutationFn: submitLoginPasswordMutationFn,
  });
  const loginMfaMutation = useMutation({
    mutationKey: identityAuthMutationKeys.loginMfa,
    mutationFn: submitLoginMfaMutationFn,
  });
  const loginWebauthnStartMutation = useMutation({
    mutationKey: identityAuthMutationKeys.loginWebauthnStart,
    mutationFn: startLoginWebAuthnMutationFn,
  });

  const loading =
    loginIdentifierMutation.isPending ||
    loginPasswordMutation.isPending ||
    loginMfaMutation.isPending ||
    loginWebauthnStartMutation.isPending;

  return {
    loginIdentifierMutation,
    loginPasswordMutation,
    loginMfaMutation,
    loginWebauthnStartMutation,
    loading,
  };
}
