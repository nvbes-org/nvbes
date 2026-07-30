import { AccountPage, AccountPageHeader } from '@/components/AccountPage';
import StepUpModal from '@/components/StepUpModal';
import { Card, CardContent } from '@/components/ui/card';
import { AccountPasswordForm } from '@/pages/AccountPasswordPage.form';
import type { AccountPasswordPageModel } from '@/pages/AccountPasswordPage.types';
import { AccountPasswordSessionDialog } from '@/pages/AccountPasswordPage.sessions-dialog';

export function AccountPasswordPageContent({
  accountEmail,
  confirmPassword,
  error,
  handleStepUpSuccess,
  handleSubmit,
  mutation,
  navigateBack,
  newPassword,
  setConfirmPassword,
  setNewPassword,
  setShowStepUp,
  showStepUp,
  success,
  handleRevokeOtherSessions,
  requestResetEmail,
  resetEmailMutation,
  resetEmailResendSeconds,
  resetEmailSent,
  setShowSessionPrompt,
  setShowSessionStepUp,
  showSessionPrompt,
  showSessionStepUp,
}: AccountPasswordPageModel) {
  return (
    <AccountPage>
      <AccountPageHeader title="Mot de passe" onBack={navigateBack} />

      <Card>
        <CardContent>
          <AccountPasswordForm
            accountEmail={accountEmail}
            confirmPassword={confirmPassword}
            error={error}
            handleSubmit={handleSubmit}
            mutation={mutation}
            newPassword={newPassword}
            setConfirmPassword={setConfirmPassword}
            setNewPassword={setNewPassword}
            success={success}
            requestResetEmail={requestResetEmail}
            resetEmailMutation={resetEmailMutation}
            resetEmailResendSeconds={resetEmailResendSeconds}
            resetEmailSent={resetEmailSent}
          />
        </CardContent>
      </Card>

      <StepUpModal
        open={showStepUp}
        onOpenChange={setShowStepUp}
        onSuccess={handleStepUpSuccess}
        onCancel={() => setShowStepUp(false)}
        description="Pour modifier votre mot de passe, veuillez confirmer votre identité."
        purpose="password_change"
      />

      <AccountPasswordSessionDialog
        open={showSessionPrompt}
        onKeepSessions={() => setShowSessionPrompt(false)}
        onRevokeSessions={() => {
          setShowSessionPrompt(false);
          setShowSessionStepUp(true);
        }}
      />

      <StepUpModal
        open={showSessionStepUp}
        onOpenChange={setShowSessionStepUp}
        onSuccess={() => void handleRevokeOtherSessions()}
        onCancel={() => setShowSessionStepUp(false)}
        description="Confirmez votre identité pour déconnecter toutes les autres sessions."
      />
    </AccountPage>
  );
}
