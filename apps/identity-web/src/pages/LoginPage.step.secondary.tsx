import { LoginPageConsent } from './LoginPageConsent';
import { LoginPageMfaStep } from './LoginPageMfaStep';
import type { LoginPageStepContentProps } from './LoginPage.step.types';

export function LoginPageSecondaryStepContent({
  availableCount,
  error,
  handleConsentApprove,
  handleConsentCancel,
  handleMfaSubmit,
  hasRecovery,
  hasTotp,
  hasWebAuthn,
  hostedConsent,
  loading,
  mfaMethod,
  oauthRequest,
  recoveryCode,
  resetToIdentifier,
  setError,
  setMfaMethod,
  setRecoveryCode,
  setTotpCode,
  step,
  totpCode,
}: Pick<
  LoginPageStepContentProps,
  | 'availableCount'
  | 'error'
  | 'handleConsentApprove'
  | 'handleConsentCancel'
  | 'handleMfaSubmit'
  | 'hasRecovery'
  | 'hasTotp'
  | 'hasWebAuthn'
  | 'hostedConsent'
  | 'loading'
  | 'mfaMethod'
  | 'oauthRequest'
  | 'recoveryCode'
  | 'resetToIdentifier'
  | 'setError'
  | 'setMfaMethod'
  | 'setRecoveryCode'
  | 'setTotpCode'
  | 'step'
  | 'totpCode'
>) {
  if (step === 'consent') {
    return (
      <LoginPageConsent
        clientName={hostedConsent?.client.name}
        scope={hostedConsent?.scope ?? oauthRequest?.scope}
        error={error}
        onApprove={handleConsentApprove}
        onCancel={handleConsentCancel}
      />
    );
  }

  return (
    <LoginPageMfaStep
      error={error}
      loading={loading}
      mfaMethod={mfaMethod}
      hasTotp={hasTotp}
      hasWebAuthn={hasWebAuthn}
      hasRecovery={hasRecovery}
      availableCount={availableCount}
      totpCode={totpCode}
      recoveryCode={recoveryCode}
      onTotpCodeChange={setTotpCode}
      onRecoveryCodeChange={setRecoveryCode}
      onMfaMethodSelect={setMfaMethod}
      onMfaSubmit={handleMfaSubmit}
      onBackToMethodSelect={() => {
        setMfaMethod(null);
        setError(null);
      }}
      onResetToIdentifier={resetToIdentifier}
    />
  );
}
