import { PasswordStrengthMeter, RegionSelect } from './RegisterPage.shared';
import { RegisterPageStepOne } from './RegisterPageStepOne';
import { RegisterPageStepTwo } from './RegisterPageStepTwo';
import type { useRegisterPage } from './useRegisterPage';

type RegisterPageContentProps = ReturnType<typeof useRegisterPage>;

export function RegisterPageContent({
  birthdate,
  canProceedFromStep1,
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
}: RegisterPageContentProps) {
  if (step === 1) {
    return (
      <RegisterPageStepOne
        firstname={firstname}
        lastname={lastname}
        username={username}
        birthdate={birthdate}
        email={email}
        password={password}
        minBirthdate={minBirthdate}
        maxBirthdate={maxBirthdate}
        canProceed={canProceedFromStep1}
        onFirstnameChange={setFirstname}
        onLastnameChange={setLastname}
        onUsernameChange={setUsername}
        onBirthdateChange={setBirthdate}
        onEmailChange={setEmail}
        onPasswordChange={setPassword}
        onSubmit={(event) => {
          event.preventDefault();
          handleStep1Next();
        }}
        passwordStrength={<PasswordStrengthMeter password={password} />}
      />
    );
  }

  return (
    <RegisterPageStepTwo
      workspaceName={workspaceName}
      selectedRegion={selectedRegion}
      detectedRegion={detectedRegion}
      detectedReliability={detectedReliability}
      regionLoading={regionLoading}
      supportedRegions={supportedRegions}
      error={error}
      loading={loading}
      onWorkspaceNameChange={setWorkspaceName}
      onRegionChange={setSelectedRegion}
      onBack={handleStep2Back}
      onSubmit={handleSubmit}
      regionSelect={(props) => <RegionSelect {...props} />}
    />
  );
}
