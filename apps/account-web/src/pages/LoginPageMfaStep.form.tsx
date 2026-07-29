import { useEffect, useRef } from 'react';

import { Alert, AlertDescription } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import type { LoginPageMfaStepProps } from './LoginPageMfaStep.types';

export function LoginPageMfaMethodForm({
  error,
  loading,
  mfaMethod,
  totpCode,
  recoveryCode,
  onTotpCodeChange,
  onRecoveryCodeChange,
  onMfaSubmit,
  onBackToMethodSelect,
}: Pick<
  LoginPageMfaStepProps,
  | 'error'
  | 'loading'
  | 'mfaMethod'
  | 'totpCode'
  | 'recoveryCode'
  | 'onTotpCodeChange'
  | 'onRecoveryCodeChange'
  | 'onMfaSubmit'
  | 'onBackToMethodSelect'
>) {
  const formRef = useRef<HTMLFormElement>(null);
  const webauthnAutoSubmittedRef = useRef(false);

  useEffect(() => {
    if (mfaMethod !== 'webauthn') {
      webauthnAutoSubmittedRef.current = false;
      return;
    }

    if (loading || webauthnAutoSubmittedRef.current) return;

    webauthnAutoSubmittedRef.current = true;
    formRef.current?.requestSubmit();
  }, [loading, mfaMethod]);

  return (
    <form ref={formRef} onSubmit={onMfaSubmit} className="flex flex-col gap-5">
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
          La vérification avec votre clé de sécurité va démarrer automatiquement.
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
