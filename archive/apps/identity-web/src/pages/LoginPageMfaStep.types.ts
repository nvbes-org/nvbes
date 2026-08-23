import type { MfaMethod } from './LoginPage.mfa';

export type LoginPageMfaStepProps = {
  error: string | null;
  loading: boolean;
  mfaMethod: MfaMethod | null;
  hasTotp: boolean;
  hasWebAuthn: boolean;
  hasRecovery: boolean;
  availableCount: number;
  totpCode: string;
  recoveryCode: string;
  onTotpCodeChange: (value: string) => void;
  onRecoveryCodeChange: (value: string) => void;
  onMfaMethodSelect: (method: MfaMethod) => void;
  onMfaSubmit: (event: React.SubmitEvent<HTMLFormElement>) => void;
  onBackToMethodSelect: () => void;
  onResetToIdentifier: () => void;
};
