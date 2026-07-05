import { KeyRound } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import type { StepUpMethod } from './useStepUpForm';

export function StepUpMethodFields({
  method,
  password,
  totpCode,
  recoveryCode,
  loading,
  onPasswordChange,
  onTotpCodeChange,
  onRecoveryCodeChange,
  onWebAuthnClick,
}: {
  method: StepUpMethod;
  password: string;
  totpCode: string;
  recoveryCode: string;
  loading: boolean;
  onPasswordChange: (value: string) => void;
  onTotpCodeChange: (value: string) => void;
  onRecoveryCodeChange: (value: string) => void;
  onWebAuthnClick: () => void;
}) {
  if (method === 'password') {
    return (
      <div className="space-y-2">
        <Label htmlFor="stepup-password" className="text-xs">
          Mot de passe
        </Label>
        <Input
          id="stepup-password"
          type="password"
          placeholder="Saisissez votre mot de passe"
          value={password}
          onChange={(event) => onPasswordChange(event.target.value)}
          className="text-xs"
          required
          autoFocus
        />
      </div>
    );
  }

  if (method === 'totp') {
    return (
      <div className="space-y-2">
        <Label htmlFor="stepup-totp" className="text-xs">
          Code d&apos;authentification
        </Label>
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
      </div>
    );
  }

  if (method === 'recovery') {
    return (
      <div className="space-y-2">
        <Label htmlFor="stepup-recovery" className="text-xs">
          Code de récupération
        </Label>
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
      </div>
    );
  }

  return (
    <div className="flex flex-col items-center justify-center gap-3 py-3">
      <p className="text-center text-xs text-muted-foreground">
        Utilisez votre authentification biométrique ou clé de sécurité physique.
      </p>
      <Button
        type="button"
        onClick={onWebAuthnClick}
        disabled={loading}
        className="w-full gap-2 text-xs"
      >
        <KeyRound className="size-4" />
        {loading ? 'Attente de la clé...' : 'Déclencher la clé de sécurité'}
      </Button>
    </div>
  );
}
