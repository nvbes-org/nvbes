import type { StepUpMethod } from './MfaPage.shared';

export type MfaPageStepUpDialogProps = {
  open: boolean;
  stepUpMethod: StepUpMethod | null;
  hasTotp: boolean;
  hasWebAuthn: boolean;
  hasRecovery: boolean;
  stepUpPassword: string;
  stepUpTotpCode: string;
  stepUpRecoveryCode: string;
  stepUpLoading: boolean;
  stepUpError: string | null;
  onOpenChange: (open: boolean) => void;
  onMethodSelect: (method: StepUpMethod) => void;
  onPasswordChange: (value: string) => void;
  onTotpCodeChange: (value: string) => void;
  onRecoveryCodeChange: (value: string) => void;
  onBack: () => void;
  onSubmit: (event: React.SubmitEvent<HTMLFormElement>) => void;
};
