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
    maxBirthdate,
    minBirthdate,
    password,
    setBirthdate,
    setEmail,
    setFirstname,
    setLastname,
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
    onSuccess: ({ email: successEmail, resendAvailableAt }) => {
      void navigate({
        to: '/verify',
        state: (state) => ({
          ...state,
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
    loading,
    maxBirthdate,
    minBirthdate,
    password,
    regionLoading,
    selectedRegion,
    setBirthdate,
    setEmail,
    setFirstname,
    setLastname,
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
