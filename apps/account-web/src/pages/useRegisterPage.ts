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
    step,
    username,
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

  const { emailAlreadyExists, error, handleSubmit, loading, resetError } = useRegisterPageSubmit({
    oauthRequest,
    detectedRegion,
    selectedRegion,
    firstname,
    lastname,
    username,
    birthdate,
    email,
    password,
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

  const handleEditEmail = () => {
    resetError();
    setStep(1);
  };

  return {
    birthdate,
    canProceedFromStep1,
    checkingAuth,
    detectedRegion,
    detectedReliability,
    email,
    emailAlreadyExists,
    error,
    firstname,
    handleStep1Next,
    handleStep2Back,
    handleEditEmail,
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
    step,
    supportedRegions,
    username,
  };
}
