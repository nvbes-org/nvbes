import { Fingerprint, RotateCcw, ShieldAlert } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Field, FieldLabel } from '@/components/ui/field';
import { Input } from '@/components/ui/input';
import type { StepUpMethod, WebAuthnStatus } from './useStepUpForm';

export function StepUpMethodFields({
  method,
  webauthnStatus = 'idle',
  password,
  totpCode,
  recoveryCode,
  emailCode,
  emailCodeSent,
  loading,
  onPasswordChange,
  onTotpCodeChange,
  onRecoveryCodeChange,
  onEmailCodeChange,
  onSendEmailCode,
  onWebAuthnClick,
}: {
  method: StepUpMethod;
  webauthnStatus?: WebAuthnStatus;
  password: string;
  totpCode: string;
  recoveryCode: string;
  emailCode: string;
  emailCodeSent: boolean;
  loading: boolean;
  onPasswordChange: (value: string) => void;
  onTotpCodeChange: (value: string) => void;
  onRecoveryCodeChange: (value: string) => void;
  onEmailCodeChange: (value: string) => void;
  onSendEmailCode: () => void;
  onWebAuthnClick: () => void;
}) {
  if (method === 'password') {
    return (
      <Field>
        <FieldLabel htmlFor="stepup-password" className="text-xs">
          Mot de passe
        </FieldLabel>
        <Input
          id="stepup-password"
          name="password"
          type="password"
          autoComplete="current-password"
          placeholder="Saisissez votre mot de passe"
          value={password}
          onChange={(event) => onPasswordChange(event.target.value)}
          spellCheck={false}
          className="text-xs"
          required
          autoFocus
        />
      </Field>
    );
  }

  if (method === 'totp') {
    return (
      <Field>
        <FieldLabel htmlFor="stepup-totp" className="text-xs">
          Code d&apos;authentification
        </FieldLabel>
        <Input
          id="stepup-totp"
          type="text"
          inputMode="numeric"
          placeholder="000000"
          maxLength={6}
          value={totpCode}
          onChange={(event) => onTotpCodeChange(event.target.value)}
          className="font-mono text-center text-xs tracking-widest"
          required
          autoFocus
        />
      </Field>
    );
  }

  if (method === 'recovery') {
    return (
      <Field>
        <FieldLabel htmlFor="stepup-recovery" className="text-xs">
          Code de récupération
        </FieldLabel>
        <Input
          id="stepup-recovery"
          type="text"
          placeholder="XXXX-XXXX-XXXX"
          value={recoveryCode}
          onChange={(event) => onRecoveryCodeChange(event.target.value)}
          className="font-mono text-xs"
          required
          autoFocus
        />
      </Field>
    );
  }

  if (method === 'email') {
    return (
      <div className="space-y-3">
        <p className="text-xs text-muted-foreground">
          Le code est valable uniquement pour ce changement de mot de passe.
        </p>
        <Button
          type="button"
          variant="outline"
          className="w-full text-xs"
          disabled={loading}
          onClick={onSendEmailCode}
        >
          {emailCodeSent ? 'Renvoyer le code' : 'Envoyer un code'}
        </Button>
        {emailCodeSent && (
          <Field>
            <FieldLabel htmlFor="stepup-email-code" className="text-xs">
              Code reçu par email
            </FieldLabel>
            <Input
              id="stepup-email-code"
              type="text"
              inputMode="numeric"
              autoComplete="one-time-code"
              placeholder="000000"
              maxLength={6}
              value={emailCode}
              onChange={(event) => onEmailCodeChange(event.target.value)}
              className="font-mono text-center text-xs tracking-widest"
              required
              autoFocus
            />
          </Field>
        )}
      </div>
    );
  }

  const isPrompting =
    webauthnStatus === 'prompting' ||
    webauthnStatus === 'idle' ||
    (loading && webauthnStatus !== 'error');

  if (isPrompting) {
    return (
      <div className="flex flex-col items-center justify-center gap-3 py-6 px-4 rounded-xl border bg-muted/20 text-center animate-in fade-in zoom-in-95 duration-200">
        <div className="relative flex items-center justify-center size-14 rounded-full bg-primary/10 text-primary">
          <Fingerprint className="size-7 animate-pulse" />
          <span className="absolute inset-0 rounded-full border-2 border-primary/40 animate-ping opacity-75" />
        </div>
        <div className="space-y-1">
          <p className="text-xs font-semibold text-foreground">
            Détection de votre clé de sécurité...
          </p>
          <p className="text-[11px] text-muted-foreground leading-relaxed max-w-[280px] mx-auto">
            Veuillez patienter pendant l&apos;ouverture de la fenêtre de votre navigateur ou
            appareil.
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="flex flex-col items-center justify-center gap-3 py-5 px-4 rounded-xl border bg-destructive/5 border-destructive/20 text-center animate-in fade-in zoom-in-95 duration-200">
      <div className="flex items-center justify-center size-12 rounded-full bg-destructive/10 text-destructive">
        <ShieldAlert className="size-6" />
      </div>
      <div className="space-y-1">
        <p className="text-xs font-semibold text-destructive">
          Authentification biométrique interrompue
        </p>
        <p className="text-[11px] text-muted-foreground leading-relaxed max-w-[280px] mx-auto">
          La fenêtre a expiré ou la vérification a été annulée.
        </p>
      </div>
      <Button
        type="button"
        onClick={onWebAuthnClick}
        disabled={loading}
        className="w-full gap-2 text-xs mt-1"
      >
        <RotateCcw className="size-3.5" />
        Réessayer la clé de sécurité
      </Button>
    </div>
  );
}
