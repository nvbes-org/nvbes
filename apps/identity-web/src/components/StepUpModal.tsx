import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import {
  StepUpActions,
  StepUpError,
  StepUpMethodFields,
  StepUpMethodSelect,
} from './StepUpForm.shared';
import { useStepUpForm } from './useStepUpForm';
import type { StepUpPurpose } from '@nvbes/identity-sdk-web';

export interface StepUpModalProps {
  open: boolean;
  onOpenChange?: (open: boolean) => void;
  onSuccess: () => void;
  onCancel: () => void;
  description?: string;
  purpose?: StepUpPurpose;
}

export function StepUpModal({
  open,
  onOpenChange,
  onSuccess,
  onCancel,
  description,
  purpose,
}: StepUpModalProps) {
  const {
    error,
    allowEmail,
    canUsePassword,
    emailCode,
    emailCodeSent,
    handleSubmit,
    handleWebAuthnClick,
    hasRecovery,
    hasTotp,
    hasWebAuthn,
    loading,
    method,
    password,
    prerequisitesLoaded,
    recoveryCode,
    sendEmailCode,
    setEmailCode,
    setMethod,
    setPassword,
    setRecoveryCode,
    setTotpCode,
    totpCode,
    webauthnStatus,
  } = useStepUpForm({ open, onSuccess, purpose });

  const handleOpenChange = (newOpen: boolean) => {
    if (!newOpen) {
      onCancel();
    }
    onOpenChange?.(newOpen);
  };

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>Vérification requise</DialogTitle>
          <DialogDescription>
            {description || 'Veuillez confirmer votre identité pour continuer.'}
          </DialogDescription>
        </DialogHeader>

        {!prerequisitesLoaded ? (
          <p className="py-6 text-center text-sm text-muted-foreground">
            Chargement des méthodes de vérification...
          </p>
        ) : (
          <form onSubmit={handleSubmit} className="flex flex-col gap-4">
            <StepUpMethodSelect
              method={method}
              hasWebAuthn={hasWebAuthn}
              hasTotp={hasTotp}
              hasRecovery={hasRecovery}
              canUsePassword={canUsePassword}
              allowEmail={allowEmail}
              onMethodChange={setMethod}
            />
            <StepUpMethodFields
              method={method}
              webauthnStatus={webauthnStatus}
              password={password}
              totpCode={totpCode}
              recoveryCode={recoveryCode}
              emailCode={emailCode}
              emailCodeSent={emailCodeSent}
              loading={loading}
              onPasswordChange={setPassword}
              onTotpCodeChange={setTotpCode}
              onRecoveryCodeChange={setRecoveryCode}
              onEmailCodeChange={setEmailCode}
              onSendEmailCode={sendEmailCode}
              onWebAuthnClick={handleWebAuthnClick}
            />
            {error && method !== 'webauthn' && <StepUpError error={error} />}
            <StepUpActions method={method} loading={loading} onCancel={onCancel} />
          </form>
        )}
      </DialogContent>
    </Dialog>
  );
}

export default StepUpModal;
