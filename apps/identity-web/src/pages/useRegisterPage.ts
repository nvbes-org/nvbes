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
    setUsername,
    username,
    usernameValid,
  } = useRegisterPageState();
  const { emailAvailability, usernameAvailability } = useRegisterPageAvailability({
    email,
    emailValid,
    username,
    usernameValid,
  });
  const availabilityAllowsSubmit =
    (emailAvailability === 'available' || emailAvailability === 'error') &&
    (usernameAvailability === 'available' || usernameAvailability === 'error');
  const registrationCanSubmit = canSubmit && availabilityAllowsSubmit;

  const { emailAlreadyExists, error, handleSubmit, loading, resetError, usernameTaken } =
    useRegisterPageSubmit({
      oauthRequest,
      username,
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

  const handleUsernameChange = (value: string) => {
    resetError();
    setUsername(value);
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
    handleUsernameChange,
    legalDocumentsAccepted,
    loading,
    marketingEmailsAccepted,
    password,
    passwordValid,
    setLegalDocumentsAccepted,
    setMarketingEmailsAccepted,
    setPassword,
    username,
    usernameTaken,
    usernameAvailability,
    usernameValid,
  };
}
