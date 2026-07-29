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
        logoUrl={hostedConsent?.client.logo_url}
        description={hostedConsent?.client.description}
        supportUrl={hostedConsent?.client.support_url}
        privacyUrl={hostedConsent?.client.privacy_url}
        termsUrl={hostedConsent?.client.terms_url}
        brandColor={hostedConsent?.client.brand_color}
        customCss={hostedConsent?.client.custom_css}
        helpText={hostedConsent?.client.help_text}
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
