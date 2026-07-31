import { AuthPageShell } from '@/components/AuthPageShell';
import { LoginBrandPanel } from './LoginBrandPanel';
import type { LoginPageContentProps } from './LoginPage.content.types';
import { LoginPageFooter, LoginPageLegalLinks } from './LoginPage.forms';
import { LoginPageCard, LoginPageLoading } from './LoginPage.layout';
import { LoginPageStepContent } from './LoginPage.step';
import { LoginProgress } from './LoginProgress';

export function LoginPageContent({
  availableCount,
  checkingAuth,
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
  loading,
  identifierSubmitting,
  location,
  hostedConsent,
  isOAuthFlow,
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
  transitionDirection,
}: LoginPageContentProps) {
  if (checkingAuth) {
    return <LoginPageLoading />;
  }

  return (
    <AuthPageShell brand={<LoginBrandPanel step={step} />} footerLinks={<LoginPageLegalLinks />}>
      <LoginProgress isOAuthFlow={isOAuthFlow} step={step} />

      <LoginPageCard
        key={step}
        step={step}
        transitionDirection={transitionDirection}
        footer={
          step !== 'consent' && step !== 'chooser' ? (
            <LoginPageFooter searchStr={location.searchStr} />
          ) : null
        }
      >
        <LoginPageStepContent
          availableCount={availableCount}
          connectedAccounts={connectedAccounts}
          email={email}
          error={error}
          handleAccountSelect={handleAccountSelect}
          handleDisconnectAccount={handleDisconnectAccount}
          handleDisconnectAllAccounts={handleDisconnectAllAccounts}
          handleConsentApprove={handleConsentApprove}
          handleConsentCancel={handleConsentCancel}
          handleIdentifierSubmit={handleIdentifierSubmit}
          handleMfaSubmit={handleMfaSubmit}
          handlePasswordSubmit={handlePasswordSubmit}
          handleUseAnotherAccount={handleUseAnotherAccount}
          hasRecovery={hasRecovery}
          hasTotp={hasTotp}
          hasWebAuthn={hasWebAuthn}
          loading={loading}
          identifierSubmitting={identifierSubmitting}
          hostedConsent={hostedConsent}
          mfaMethod={mfaMethod}
          oauthRequest={oauthRequest}
          password={password}
          recoveryCode={recoveryCode}
          resetToIdentifier={resetToIdentifier}
          setEmail={setEmail}
          setError={setError}
          setMfaMethod={setMfaMethod}
          setPassword={setPassword}
          setRecoveryCode={setRecoveryCode}
          setTotpCode={setTotpCode}
          step={step}
          totpCode={totpCode}
        />
      </LoginPageCard>
    </AuthPageShell>
  );
}
