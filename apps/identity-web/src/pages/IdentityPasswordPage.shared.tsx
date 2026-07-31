import { IdentityPage, IdentityPageHeader } from '@/components/IdentityPage';
import StepUpModal from '@/components/StepUpModal';
import { Card, CardContent } from '@/components/ui/card';
import { IdentityPasswordForm } from '@/pages/IdentityPasswordPage.form';
import type { IdentityPasswordPageModel } from '@/pages/IdentityPasswordPage.types';
import { IdentityPasswordSessionDialog } from '@/pages/IdentityPasswordPage.sessions-dialog';

export function IdentityPasswordPageContent({
  email,
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
}: IdentityPasswordPageModel) {
  return (
    <IdentityPage>
      <IdentityPageHeader title="Mot de passe" onBack={navigateBack} />

      <Card>
        <CardContent>
          <IdentityPasswordForm
            email={email}
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

      <IdentityPasswordSessionDialog
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
    </IdentityPage>
  );
}
