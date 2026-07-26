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

export interface StepUpModalProps {
  open: boolean;
  onOpenChange?: (open: boolean) => void;
  onSuccess: () => void;
  onCancel: () => void;
  description?: string;
}

export function StepUpModal({
  open,
  onOpenChange,
  onSuccess,
  onCancel,
  description,
}: StepUpModalProps) {
  const {
    error,
    handleSubmit,
    handleWebAuthnClick,
    hasRecovery,
    hasTotp,
    hasWebAuthn,
    loading,
    method,
    password,
    recoveryCode,
    setMethod,
    setPassword,
    setRecoveryCode,
    setTotpCode,
    totpCode,
    webauthnStatus,
  } = useStepUpForm({ open, onSuccess });

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

        <form onSubmit={handleSubmit} className="flex flex-col gap-4">
          <StepUpMethodSelect
            method={method}
            hasWebAuthn={hasWebAuthn}
            hasTotp={hasTotp}
            hasRecovery={hasRecovery}
            onMethodChange={setMethod}
          />
          <StepUpMethodFields
            method={method}
            webauthnStatus={webauthnStatus}
            password={password}
            totpCode={totpCode}
            recoveryCode={recoveryCode}
            loading={loading}
            onPasswordChange={setPassword}
            onTotpCodeChange={setTotpCode}
            onRecoveryCodeChange={setRecoveryCode}
            onWebAuthnClick={handleWebAuthnClick}
          />
          {error && method !== 'webauthn' && <StepUpError error={error} />}
          <StepUpActions method={method} loading={loading} onCancel={onCancel} />
        </form>
      </DialogContent>
    </Dialog>
  );
}

export default StepUpModal;
