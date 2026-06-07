import { LoginPageChooser } from './LoginPageChooser';
import { LoginPageIdentifierForm, LoginPagePasswordForm } from './LoginPage.forms';
import type { LoginPageStepContentProps } from './LoginPage.step.types';

export function LoginPagePrimaryStepContent({
  connectedAccounts,
  email,
  error,
  handleAccountSelect,
  handleIdentifierSubmit,
  handlePasswordSubmit,
  handleUseAnotherAccount,
  identifierSubmitting,
  loading,
  password,
  resetToIdentifier,
  setEmail,
  setPassword,
  step,
}: Pick<
  LoginPageStepContentProps,
  | 'connectedAccounts'
  | 'email'
  | 'error'
  | 'handleAccountSelect'
  | 'handleIdentifierSubmit'
  | 'handlePasswordSubmit'
  | 'handleUseAnotherAccount'
  | 'identifierSubmitting'
  | 'loading'
  | 'password'
  | 'resetToIdentifier'
  | 'setEmail'
  | 'setPassword'
  | 'step'
>) {
  if (step === 'chooser') {
    return (
      <LoginPageChooser
        accounts={connectedAccounts}
        onAccountSelect={handleAccountSelect}
        onUseAnotherAccount={handleUseAnotherAccount}
      />
    );
  }

  if (step === 'identifier') {
    return (
      <LoginPageIdentifierForm
        email={email}
        error={error}
        loading={identifierSubmitting || loading}
        onEmailChange={setEmail}
        onSubmit={handleIdentifierSubmit}
      />
    );
  }

  return (
    <LoginPagePasswordForm
      email={email}
      password={password}
      error={error}
      loading={loading}
      onPasswordChange={setPassword}
      onSubmit={handlePasswordSubmit}
      onBack={resetToIdentifier}
    />
  );
}
