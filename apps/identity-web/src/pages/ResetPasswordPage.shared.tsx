import { AuthFooterLink } from '@/components/AuthFooterLink';
import { StandaloneAuthCard } from '@/components/StandaloneAuthCard';
import { ResetPasswordForm } from './ResetPasswordPage.form';
import { ResetPasswordSuccessState } from './ResetPasswordPage.success';
import type { ResetPasswordPageModel } from './ResetPasswordPage.types';

export function ResetPasswordPageCard({
  tokenFromLink,
  token,
  password,
  confirmPassword,
  error,
  success,
  isPending,
  navigateToLogin,
  setToken,
  setPassword,
  setConfirmPassword,
  handleSubmit,
}: ResetPasswordPageModel) {
  return (
    <StandaloneAuthCard
      title="Nouveau mot de passe"
      description="Choisissez un nouveau mot de passe pour votre compte."
      onBack={navigateToLogin}
      footer={<AuthFooterLink to="/forgot-password" label="Renvoyer le lien" />}
    >
      {success ? (
        <ResetPasswordSuccessState navigateToLogin={navigateToLogin} />
      ) : (
        <ResetPasswordForm
          tokenFromLink={tokenFromLink}
          token={token}
          password={password}
          confirmPassword={confirmPassword}
          error={error}
          isPending={isPending}
          setToken={setToken}
          setPassword={setPassword}
          setConfirmPassword={setConfirmPassword}
          handleSubmit={handleSubmit}
        />
      )}
    </StandaloneAuthCard>
  );
}
