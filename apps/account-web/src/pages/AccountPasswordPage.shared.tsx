import { ArrowLeft } from 'lucide-react';
import StepUpModal from '@/components/StepUpModal';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { AccountPasswordForm } from '@/pages/AccountPasswordPage.form';
import type { AccountPasswordPageModel } from '@/pages/AccountPasswordPage.types';

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
}: AccountPasswordPageModel) {
  return (
    <div className="flex flex-col gap-6 animate-fade-slide-up [animation-delay:0ms]">
      <div>
        <Button
          variant="ghost"
          size="sm"
          className="mb-3 -ml-1 text-muted-foreground"
          onClick={navigateBack}
        >
          <ArrowLeft data-icon="inline-start" />
          Retour
        </Button>
        <h1 className="text-3xl font-heading font-semibold">Mot de passe</h1>
      </div>

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
          />
        </CardContent>
      </Card>

      <StepUpModal
        open={showStepUp}
        onOpenChange={setShowStepUp}
        onSuccess={handleStepUpSuccess}
        onCancel={() => setShowStepUp(false)}
        description="Pour modifier votre mot de passe, veuillez confirmer votre identité."
      />
    </div>
  );
}
