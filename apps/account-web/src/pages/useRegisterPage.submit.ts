import { clientErrorMessage } from '@nvbes/web-runtime';
import { useMutation } from '@tanstack/react-query';
import { type FormEvent } from 'react';
import { identityAuthMutationKeys, submitRegisterMutationFn } from '../identity.auth.queries';
import { resolvePowChallenge } from '../identity.auth.pow';
import { accountServiceBaseUrl } from '../identity.http';
import { savePendingOAuthAuthorizeRequest } from '../identity.oauth';
import { trackEvent } from '../identity.analytics';

interface RegisterSubmitArgs {
  oauthRequest: ReturnType<typeof import('../identity.oauth').readOAuthAuthorizeRequest>;
  detectedRegion: string | null;
  selectedRegion: string;
  firstname: string;
  lastname: string;
  username: string;
  birthdate: string;
  email: string;
  password: string;
  workspaceName: string;
  canProceedFromStep1: boolean;
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
  detectedRegion,
  selectedRegion,
  firstname,
  lastname,
  username,
  birthdate,
  email,
  password,
  workspaceName,
  canProceedFromStep1,
  legalDocumentsAccepted,
  marketingEmailsAccepted,
  onSuccess,
}: RegisterSubmitArgs) {
  const registerMutation = useMutation({
    mutationKey: identityAuthMutationKeys.register,
    mutationFn: submitRegisterMutationFn,
  });

  const handleSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();

    if (!canProceedFromStep1 || !legalDocumentsAccepted) {
      return;
    }

    try {
      trackEvent('auth.signup_started', {
        country: selectedRegion || detectedRegion || undefined,
      });

      const result = await registerMutation.mutateAsync({
        data: {
          email,
          firstname: firstname.trim(),
          lastname: lastname.trim(),
          username,
          birthdate: birthdate || undefined,
          password,
          workspace_name: workspaceName,
          region: selectedRegion || undefined,
          legal_documents_accepted: legalDocumentsAccepted,
          marketing_emails_accepted: marketingEmailsAccepted,
        },
        ...(await resolvePowChallenge(accountServiceBaseUrl)),
      });

      trackEvent('auth.signup_completed', {
        country: selectedRegion || detectedRegion || undefined,
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
    error: registerMutation.error
      ? clientErrorMessage(registerMutation.error, "Échec de l'inscription.")
      : null,
    handleSubmit,
    loading: registerMutation.isPending,
  };
}
