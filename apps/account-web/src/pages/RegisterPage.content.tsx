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
}: RegisterPageContentProps) {
  if (step === 1) {
    return (
      <RegisterPageStepOne
        firstname={firstname}
        lastname={lastname}
        username={username}
        birthdate={birthdate}
        email={email}
        emailError={emailAlreadyExists}
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
      selectedRegion={selectedRegion}
      detectedRegion={detectedRegion}
      detectedReliability={detectedReliability}
      regionLoading={regionLoading}
      supportedRegions={supportedRegions}
      error={error}
      emailAlreadyExists={emailAlreadyExists}
      legalDocumentsAccepted={legalDocumentsAccepted}
      loading={loading}
      marketingEmailsAccepted={marketingEmailsAccepted}
      onLegalDocumentsAcceptedChange={setLegalDocumentsAccepted}
      onMarketingEmailsAcceptedChange={setMarketingEmailsAccepted}
      onRegionChange={setSelectedRegion}
      onBack={handleStep2Back}
      onEditEmail={handleEditEmail}
      onSubmit={handleSubmit}
      regionSelect={(props) => <RegionSelect {...props} />}
    />
  );
}
