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
  hasEmail,
  hasTotp,
  hasWebAuthn,
  hostedConsent,
  identifierSubmitting,
  loading,
  loginStateToken,
  mfaMethod,
  oauthRequest,
  password,
  emailCode,
  recoveryCode,
  resetToIdentifier,
  resendLoginMfaEmailCode,
  setEmail,
  setError,
  setMfaMethod,
  setPassword,
  setEmailCode,
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
      hasEmail={hasEmail}
      hasTotp={hasTotp}
      hasWebAuthn={hasWebAuthn}
      hostedConsent={hostedConsent}
      loading={loading}
      loginStateToken={loginStateToken}
      mfaMethod={mfaMethod}
      oauthRequest={oauthRequest}
      emailCode={emailCode}
      recoveryCode={recoveryCode}
      resetToIdentifier={resetToIdentifier}
      resendLoginMfaEmailCode={resendLoginMfaEmailCode}
      setError={setError}
      setMfaMethod={setMfaMethod}
      setEmailCode={setEmailCode}
      setRecoveryCode={setRecoveryCode}
      setTotpCode={setTotpCode}
      step={step}
      totpCode={totpCode}
    />
  );
}
