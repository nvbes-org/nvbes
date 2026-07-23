import { ArrowLeft } from 'lucide-react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { AccountPasswordForm } from '@/pages/AccountPasswordPage.form';
import { AccountPasswordRecoveryCard } from '@/pages/AccountPasswordPage.recovery';
import type { AccountPasswordPageModel } from '@/pages/AccountPasswordPage.types';

export function AccountPasswordPageContent({
  confirmPassword,
  currentPassword,
  error,
  handleSubmit,
  mutation,
  navigateBack,
  newPassword,
  setConfirmPassword,
  setCurrentPassword,
  setNewPassword,
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
        <h1 className="text-xl font-heading font-semibold">Mot de passe</h1>
        <p className="mt-1 text-sm text-muted-foreground">Modifier votre mot de passe.</p>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Changer le mot de passe</CardTitle>
          <CardDescription>
            Utilisez un mot de passe fort et unique que vous n&apos;utilisez pas ailleurs.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <AccountPasswordForm
            confirmPassword={confirmPassword}
            currentPassword={currentPassword}
            error={error}
            handleSubmit={handleSubmit}
            mutation={mutation}
            newPassword={newPassword}
            setConfirmPassword={setConfirmPassword}
            setCurrentPassword={setCurrentPassword}
            setNewPassword={setNewPassword}
            success={success}
          />
        </CardContent>
      </Card>

      <AccountPasswordRecoveryCard />
    </div>
  );
}
