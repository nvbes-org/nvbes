import { KeyRoundIcon, ShieldAlertIcon, ShieldCheckIcon } from 'lucide-react';
import { Button } from '@/components/ui/button';
import type { LoginPageMfaStepProps } from './LoginPageMfaStep.types';

export function LoginPageMfaMethodChoices({
  hasTotp,
  hasWebAuthn,
  hasRecovery,
  availableCount,
  onMfaMethodSelect,
}: Pick<
  LoginPageMfaStepProps,
  'hasTotp' | 'hasWebAuthn' | 'hasRecovery' | 'availableCount' | 'onMfaMethodSelect'
>) {
  return (
    <div className="flex flex-col gap-2">
      {hasWebAuthn && (
        <Button
          variant="default"
          className="w-full justify-start gap-3"
          onClick={() => onMfaMethodSelect('webauthn')}
        >
          <KeyRoundIcon className="size-4" />
          Utiliser une passkey ou une clé de sécurité
        </Button>
      )}
      {hasTotp && (
        <Button
          variant="outline"
          className="w-full justify-start gap-3"
          onClick={() => onMfaMethodSelect('totp')}
        >
          <ShieldCheckIcon className="size-4 text-muted-foreground" />
          Utiliser le code TOTP de secours
        </Button>
      )}
      {hasRecovery && (
        <Button
          variant="outline"
          className="w-full justify-start gap-3"
          onClick={() => onMfaMethodSelect('recovery')}
        >
          <ShieldAlertIcon className="size-4 text-muted-foreground" />
          Code de récupération
        </Button>
      )}
      {availableCount === 0 && (
        <p className="py-4 text-center text-sm text-muted-foreground">
          Aucun facteur MFA n&apos;a été proposé pour cette session.
        </p>
      )}
    </div>
  );
}
