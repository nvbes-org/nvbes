import { clientErrorMessage } from '@nvbes/web-runtime';
import { useMutation } from '@tanstack/react-query';
import { type SubmitEvent } from 'react';
import { identityAuthMutationKeys, submitRegisterMutationFn } from '../identity.auth.queries';
import { resolvePowChallenge } from '../identity.auth.pow';
import { identityServiceBaseUrl } from '../identity.http';
import { savePendingOAuthAuthorizeRequest } from '../identity.oauth';
import { trackEvent } from '../identity.analytics';
import { isEmailAlreadyExistsError, isUsernameTakenError } from './RegisterPage.errors';

interface RegisterSubmitArgs {
  oauthRequest: ReturnType<typeof import('../identity.oauth').readOAuthAuthorizeRequest>;
  username: string;
  email: string;
  password: string;
  canSubmit: boolean;
  legalDocumentsAccepted: boolean;
  marketingEmailsAccepted: boolean;
  onSuccess: (args: {
    accountName: string | null;
    email: string;
    resendAvailableAt: string | null;
  }) => void;
}

export function useRegisterPageSubmit({
  oauthRequest,
  username,
  email,
  password,
  canSubmit,
  legalDocumentsAccepted,
  marketingEmailsAccepted,
  onSuccess,
}: RegisterSubmitArgs) {
  const registerMutation = useMutation({
    mutationKey: identityAuthMutationKeys.register,
    mutationFn: submitRegisterMutationFn,
  });

  const handleSubmit = async (event: SubmitEvent<HTMLFormElement>) => {
    event.preventDefault();

    if (!canSubmit || !legalDocumentsAccepted) {
      return;
    }

    try {
      trackEvent('auth.signup_started');

      const result = await registerMutation.mutateAsync({
        data: {
          email,
          username: username.trim(),
          password,
          legal_documents_accepted: legalDocumentsAccepted,
          marketing_emails_accepted: marketingEmailsAccepted,
        },
        ...(await resolvePowChallenge(identityServiceBaseUrl)),
      });

      if (oauthRequest) {
        savePendingOAuthAuthorizeRequest(oauthRequest);
      }

      const accountName = (result.user?.username ?? username).trim() || null;
      onSuccess({
        accountName,
        email,
        resendAvailableAt: result.verification_resend_available_at,
      });
    } catch {
      // error exposed through registerMutation.error
    }
  };

  return {
    emailAlreadyExists: isEmailAlreadyExistsError(registerMutation.error),
    error: registerMutation.error
      ? clientErrorMessage(registerMutation.error, "Échec de l'inscription.")
      : null,
    handleSubmit,
    loading: registerMutation.isPending,
    resetError: registerMutation.reset,
    usernameTaken: isUsernameTakenError(registerMutation.error),
  };
}
