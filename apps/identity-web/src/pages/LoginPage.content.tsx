import type { LoginPageContentProps } from './LoginPage.content.types';
import { LoginPageFooter } from './LoginPage.forms';
import { LoginPageCard, LoginPageLoading, LoginPageMobileBrand } from './LoginPage.layout';
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
  hasTotp,
  hasWebAuthn,
  loading,
  identifierSubmitting,
  location,
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
    <>
      <div className="flex flex-1 items-center justify-center bg-muted/30 p-4 sm:p-8">
        <div className="w-full max-w-sm animate-fade-slide-up [animation-delay:150ms]">
          <LoginPageMobileBrand />

          <LoginProgress step={step} />

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
        </div>
      </div>
    </>
  );
}
