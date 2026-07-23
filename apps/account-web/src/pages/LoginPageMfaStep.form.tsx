import { useEffect, useState } from 'react';

import { Alert, AlertDescription } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import type { LoginPageMfaStepProps } from './LoginPageMfaStep.types';

export function LoginPageMfaMethodForm({
  error,
  loading,
  loginStateToken,
  mfaMethod,
  totpCode,
  emailCode,
  recoveryCode,
  onTotpCodeChange,
  onEmailCodeChange,
  onRecoveryCodeChange,
  onMfaSubmit,
  onBackToMethodSelect,
  onResendEmailCode,
}: Pick<
  LoginPageMfaStepProps,
  | 'error'
  | 'loading'
  | 'loginStateToken'
  | 'mfaMethod'
  | 'totpCode'
  | 'emailCode'
  | 'recoveryCode'
  | 'onTotpCodeChange'
  | 'onEmailCodeChange'
  | 'onRecoveryCodeChange'
  | 'onMfaSubmit'
  | 'onBackToMethodSelect'
  | 'onResendEmailCode'
>) {
  const [resending, setResending] = useState(false);
  const [resendInSeconds, setResendInSeconds] = useState(0);
  const [resendError, setResendError] = useState<string | null>(null);

  useEffect(() => {
    if (resendInSeconds <= 0) return;
    const timer = window.setInterval(() => {
      setResendInSeconds((value) => Math.max(0, value - 1));
    }, 1000);
    return () => window.clearInterval(timer);
  }, [resendInSeconds]);

  async function handleResendEmailCode() {
    if (!loginStateToken || resending || resendInSeconds > 0) return;
    setResending(true);
    setResendError(null);
    try {
      await onResendEmailCode();
      setResendInSeconds(30);
    } catch (error) {
      setResendError(error instanceof Error ? error.message : 'Le code n’a pas pu être renvoyé.');
    } finally {
      setResending(false);
    }
  }

  return (
    <form onSubmit={onMfaSubmit} className="flex flex-col gap-5">
      {mfaMethod === 'totp' && (
        <div className="flex flex-col gap-2">
          <Label htmlFor="totp-code">Code d&apos;authentification</Label>
          <Input
            id="totp-code"
            type="text"
            inputMode="numeric"
            autoComplete="one-time-code"
            placeholder="000000"
            maxLength={6}
            value={totpCode}
            onChange={(event) => onTotpCodeChange(event.target.value)}
            required
            autoFocus
          />
          <Button
            type="button"
            variant="link"
            className="self-start px-0"
            onClick={() => void handleResendEmailCode()}
            disabled={!loginStateToken || resending || resendInSeconds > 0 || loading}
          >
            {resending
              ? 'Envoi en cours...'
              : resendInSeconds > 0
                ? `Renvoyer dans ${resendInSeconds}s`
                : 'Renvoyer le code'}
          </Button>
          {resendError && <p className="text-sm text-destructive">{resendError}</p>}
        </div>
      )}

      {mfaMethod === 'email' && (
        <div className="flex flex-col gap-2">
          <Label htmlFor="email-mfa-code">Code reçu par email</Label>
          <Input
            id="email-mfa-code"
            type="text"
            inputMode="numeric"
            autoComplete="one-time-code"
            placeholder="000000"
            maxLength={6}
            value={emailCode}
            onChange={(event) => onEmailCodeChange(event.target.value)}
            required
            autoFocus
          />
        </div>
      )}

      {mfaMethod === 'recovery' && (
        <div className="flex flex-col gap-2">
          <Label htmlFor="recovery-code">Code de récupération</Label>
          <Input
            id="recovery-code"
            type="text"
            placeholder="XXXX-XXXX-XXXX"
            value={recoveryCode}
            onChange={(event) => onRecoveryCodeChange(event.target.value)}
            required
            autoFocus
          />
        </div>
      )}

      {mfaMethod === 'webauthn' && (
        <p className="py-4 text-center text-sm text-muted-foreground">
          Cliquez sur Valider pour utiliser votre clé de sécurité.
        </p>
      )}

      {error && (
        <Alert variant="destructive">
          <AlertDescription>{error}</AlertDescription>
        </Alert>
      )}

      <div className="flex gap-2">
        <Button
          type="button"
          variant="outline"
          className="flex-1"
          onClick={onBackToMethodSelect}
          disabled={loading}
        >
          Retour
        </Button>
        <Button type="submit" className="flex-1" disabled={loading}>
          {loading ? 'Vérification...' : 'Valider'}
        </Button>
      </div>
    </form>
  );
}
