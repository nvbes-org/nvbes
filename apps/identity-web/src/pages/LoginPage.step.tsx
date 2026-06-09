import { LoginPagePrimaryStepContent } from './LoginPage.step.primary';
import { LoginPageSecondaryStepContent } from './LoginPage.step.secondary';
import type { LoginPageStepContentProps } from './LoginPage.step.types';

export function LoginPageStepContent({
  availableCount,
  connectedAccounts,
  email,
  error,
  handleAccountSelect,
  handleDisconnectAccount,
  handleDisconnectAllAccounts,
  handleConsentApprove,
  handleConsentCancel,
  handleIdentifierSubmit,
  handleMfaSubmit,
  handlePasswordSubmit,
  handleUseAnotherAccount,
  hasRecovery,
  hasTotp,
  hasWebAuthn,
  identifierSubmitting,
  loading,
  mfaMethod,
  oauthRequest,
  password,
  recoveryCode,
  resetToIdentifier,
  setEmail,
  setError,
  setMfaMethod,
  setPassword,
  setRecoveryCode,
  setTotpCode,
  step,
  totpCode,
}: LoginPageStepContentProps) {
  if (step === 'chooser' || step === 'identifier' || step === 'password') {
    return (
      <LoginPagePrimaryStepContent
        connectedAccounts={connectedAccounts}
        email={email}
        error={error}
        handleAccountSelect={handleAccountSelect}
        handleDisconnectAccount={handleDisconnectAccount}
        handleDisconnectAllAccounts={handleDisconnectAllAccounts}
        handleIdentifierSubmit={handleIdentifierSubmit}
        handlePasswordSubmit={handlePasswordSubmit}
        handleUseAnotherAccount={handleUseAnotherAccount}
        identifierSubmitting={step === 'identifier' && identifierSubmitting}
        loading={loading}
        password={password}
        resetToIdentifier={resetToIdentifier}
        setEmail={setEmail}
        setPassword={setPassword}
        step={step}
      />
    );
  }

  return (
    <LoginPageSecondaryStepContent
      availableCount={availableCount}
      error={error}
      handleConsentApprove={handleConsentApprove}
      handleConsentCancel={handleConsentCancel}
      handleMfaSubmit={handleMfaSubmit}
      hasRecovery={hasRecovery}
      hasTotp={hasTotp}
      hasWebAuthn={hasWebAuthn}
      loading={loading}
      mfaMethod={mfaMethod}
      oauthRequest={oauthRequest}
      recoveryCode={recoveryCode}
      resetToIdentifier={resetToIdentifier}
      setError={setError}
      setMfaMethod={setMfaMethod}
      setRecoveryCode={setRecoveryCode}
      setTotpCode={setTotpCode}
      step={step}
      totpCode={totpCode}
    />
  );
}
