import { useNavigate } from '@tanstack/react-router';
import { useRegisterPageBootstrap } from './useRegisterPage.bootstrap';
import { useRegisterPageRegions } from './useRegisterPage.regions';
import { useRegisterPageState } from './useRegisterPage.state';
import { useRegisterPageSubmit } from './useRegisterPage.submit';

export function useRegisterPage() {
  const navigate = useNavigate();
  const { checkingAuth, oauthRequest } = useRegisterPageBootstrap();
  const {
    birthdate,
    canProceedFromStep1,
    email,
    firstname,
    lastname,
    legalDocumentsAccepted,
    marketingEmailsAccepted,
    maxBirthdate,
    minBirthdate,
    password,
    setBirthdate,
    setEmail,
    setFirstname,
    setLastname,
    setLegalDocumentsAccepted,
    setMarketingEmailsAccepted,
    setPassword,
    setStep,
    setUsername,
    setWorkspaceName,
    step,
    username,
    workspaceName,
  } = useRegisterPageState();
  const {
    detectedRegion,
    detectedReliability,
    regionLoading,
    selectedRegion,
    setSelectedRegion,
    supportedRegions,
  } = useRegisterPageRegions();

  const handleStep1Next = () => {
    if (!canProceedFromStep1) {
      return;
    }
    setStep(2);
  };

  const handleStep2Back = () => {
    setStep(1);
  };

  const { error, handleSubmit, loading } = useRegisterPageSubmit({
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

  return {
    birthdate,
    canProceedFromStep1,
    checkingAuth,
    detectedRegion,
    detectedReliability,
    email,
    error,
    firstname,
    handleStep1Next,
    handleStep2Back,
    handleSubmit,
    lastname,
    legalDocumentsAccepted,
    loading,
    marketingEmailsAccepted,
    maxBirthdate,
    minBirthdate,
    password,
    regionLoading,
    selectedRegion,
    setBirthdate,
    setEmail,
    setFirstname,
    setLastname,
    setLegalDocumentsAccepted,
    setMarketingEmailsAccepted,
    setPassword,
    setSelectedRegion,
    setUsername,
    setWorkspaceName,
    step,
    supportedRegions,
    username,
    workspaceName,
  };
}
