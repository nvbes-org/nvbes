import { Fingerprint, KeyRound, ShieldAlert, Smartphone } from 'lucide-react';
import { Button } from '@/components/ui/button';
import type { MfaPageStepUpDialogProps } from './MfaPageStepUpDialog.types';

export function MfaPageStepUpMethodChoices({
  hasTotp,
  hasWebAuthn,
  hasRecovery,
  onMethodSelect,
}: Pick<MfaPageStepUpDialogProps, 'hasTotp' | 'hasWebAuthn' | 'hasRecovery' | 'onMethodSelect'>) {
  return (
    <div className="flex flex-col gap-2">
      <Button
        variant="outline"
        className="justify-start gap-3"
        onClick={() => onMethodSelect('password')}
      >
        <KeyRound className="size-4 text-muted-foreground" />
        Mot de passe
      </Button>
      {hasTotp && (
        <Button
          variant="outline"
          className="justify-start gap-3"
          onClick={() => onMethodSelect('totp')}
        >
          <Smartphone className="size-4 text-muted-foreground" />
          Code d'authentification (TOTP)
        </Button>
      )}
      {hasWebAuthn && (
        <Button
          variant="outline"
          className="justify-start gap-3"
          onClick={() => onMethodSelect('webauthn')}
        >
          <Fingerprint className="size-4 text-muted-foreground" />
          Passkey ou clé de sécurité
        </Button>
      )}
      {hasRecovery && (
        <Button
          variant="outline"
          className="justify-start gap-3"
          onClick={() => onMethodSelect('recovery')}
        >
          <ShieldAlert className="size-4 text-muted-foreground" />
          Code de récupération
        </Button>
      )}
    </div>
  );
}
