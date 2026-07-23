import type { MfaMethod } from './LoginPage.mfa';

export type LoginPageMfaStepProps = {
  error: string | null;
  loading: boolean;
  loginStateToken: string | null;
  mfaMethod: MfaMethod | null;
  hasTotp: boolean;
  hasEmail: boolean;
  hasWebAuthn: boolean;
  hasRecovery: boolean;
  availableCount: number;
  totpCode: string;
  emailCode: string;
  recoveryCode: string;
  onTotpCodeChange: (value: string) => void;
  onEmailCodeChange: (value: string) => void;
  onRecoveryCodeChange: (value: string) => void;
  onMfaMethodSelect: (method: MfaMethod) => void;
  onMfaSubmit: (event: React.SubmitEvent<HTMLFormElement>) => void;
  onBackToMethodSelect: () => void;
  onResetToIdentifier: () => void;
  onResendEmailCode: () => Promise<void>;
};
