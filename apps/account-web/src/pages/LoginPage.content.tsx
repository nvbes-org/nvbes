import type { LoginPageContentProps } from './LoginPage.content.types';
import { LoginPageFooter, LoginPageLegalLinks } from './LoginPage.forms';
import { LoginPageCard, LoginPageLoading } from './LoginPage.layout';
import { LoginProgress } from './LoginProgress';
import { LoginPageStepContent } from './LoginPage.step';

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
  hasEmail,
  hasTotp,
  hasWebAuthn,
  loading,
  loginStateToken,
  identifierSubmitting,
  location,
  hostedConsent,
  isOAuthFlow,
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
  transitionDirection,
}: LoginPageContentProps) {
  if (checkingAuth) {
    return <LoginPageLoading />;
  }

  return (
    <>
      <div className="flex flex-1 items-center justify-center bg-muted/30 p-4 sm:p-8">
        <div className="w-full max-w-sm animate-fade-slide-up [animation-delay:150ms]">
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
              hasEmail={hasEmail}
              hasTotp={hasTotp}
              hasWebAuthn={hasWebAuthn}
              loading={loading}
              loginStateToken={loginStateToken}
              identifierSubmitting={identifierSubmitting}
              hostedConsent={hostedConsent}
              mfaMethod={mfaMethod}
              oauthRequest={oauthRequest}
              password={password}
              emailCode={emailCode}
              recoveryCode={recoveryCode}
              resetToIdentifier={resetToIdentifier}
              resendLoginMfaEmailCode={resendLoginMfaEmailCode}
              setEmail={setEmail}
              setError={setError}
              setMfaMethod={setMfaMethod}
              setPassword={setPassword}
              setEmailCode={setEmailCode}
              setRecoveryCode={setRecoveryCode}
              setTotpCode={setTotpCode}
              step={step}
              totpCode={totpCode}
            />
          </LoginPageCard>
          <LoginPageLegalLinks />
        </div>
      </div>
    </>
  );
}
