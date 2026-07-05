import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import {
  MfaPageStepUpFormFields,
  MfaPageStepUpMethodChoices,
  type MfaPageStepUpDialogProps,
} from './MfaPageStepUpDialog.shared';

export function MfaPageStepUpDialog({
  open,
  stepUpMethod,
  hasTotp,
  hasWebAuthn,
  hasRecovery,
  stepUpPassword,
  stepUpTotpCode,
  stepUpRecoveryCode,
  stepUpLoading,
  stepUpError,
  onOpenChange,
  onMethodSelect,
  onPasswordChange,
  onTotpCodeChange,
  onRecoveryCodeChange,
  onBack,
  onSubmit,
}: MfaPageStepUpDialogProps) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Vérification requise</DialogTitle>
          <DialogDescription>
            Pour supprimer ce facteur, veuillez vérifier votre identité.
          </DialogDescription>
        </DialogHeader>

        {!stepUpMethod ? (
          <MfaPageStepUpMethodChoices
            hasTotp={hasTotp}
            hasWebAuthn={hasWebAuthn}
            hasRecovery={hasRecovery}
            onMethodSelect={onMethodSelect}
          />
        ) : (
          <form onSubmit={onSubmit} className="flex flex-col gap-4">
            <MfaPageStepUpFormFields
              stepUpMethod={stepUpMethod}
              stepUpPassword={stepUpPassword}
              stepUpTotpCode={stepUpTotpCode}
              stepUpRecoveryCode={stepUpRecoveryCode}
              onPasswordChange={onPasswordChange}
              onTotpCodeChange={onTotpCodeChange}
              onRecoveryCodeChange={onRecoveryCodeChange}
            />

            {stepUpError && <p className="text-sm text-destructive">{stepUpError}</p>}

            <DialogFooter>
              <Button type="button" variant="outline" onClick={onBack} disabled={stepUpLoading}>
                Retour
              </Button>
              <Button type="submit" disabled={stepUpLoading}>
                {stepUpLoading ? 'Vérification...' : 'Confirmer'}
              </Button>
            </DialogFooter>
          </form>
        )}
      </DialogContent>
    </Dialog>
  );
}
