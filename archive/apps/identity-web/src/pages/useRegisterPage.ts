import { useNavigate } from '@tanstack/react-router';
import { useRegisterPageAvailability } from './useRegisterPage.availability';
import { useRegisterPageBootstrap } from './useRegisterPage.bootstrap';
import { useRegisterPageState } from './useRegisterPage.state';
import { useRegisterPageSubmit } from './useRegisterPage.submit';

export function useRegisterPage() {
  const navigate = useNavigate();
  const { checkingAuth, oauthRequest } = useRegisterPageBootstrap();
  const {
    canSubmit,
    email,
    emailValid,
    legalDocumentsAccepted,
    marketingEmailsAccepted,
    password,
    passwordValid,
    setEmail,
    setLegalDocumentsAccepted,
    setMarketingEmailsAccepted,
    setPassword,
  } = useRegisterPageState();
  const { emailAvailability } = useRegisterPageAvailability({
    email,
    emailValid,
  });
  const availabilityAllowsSubmit =
    emailAvailability === 'available' || emailAvailability === 'error';
  const registrationCanSubmit = canSubmit && availabilityAllowsSubmit;

  const { emailAlreadyExists, error, handleSubmit, loading, resetError } = useRegisterPageSubmit({
    oauthRequest,
    email,
    password,
    canSubmit: registrationCanSubmit,
    legalDocumentsAccepted,
    marketingEmailsAccepted,
    onSuccess: ({ accountName, email: successEmail, resendAvailableAt }) => {
      void navigate({
        to: '/verify',
        state: (state) => ({
          ...state,
          accountName,
          email: successEmail,
          resendAvailableAt,
        }),
      });
    },
  });

  const handleEmailChange = (value: string) => {
    resetError();
    setEmail(value);
  };

  return {
    canSubmit: registrationCanSubmit,
    checkingAuth,
    email,
    emailAlreadyExists,
    emailAvailability,
    emailValid,
    error,
    handleEmailChange,
    handleSubmit,
    legalDocumentsAccepted,
    loading,
    marketingEmailsAccepted,
    password,
    passwordValid,
    setLegalDocumentsAccepted,
    setMarketingEmailsAccepted,
    setPassword,
  };
}
