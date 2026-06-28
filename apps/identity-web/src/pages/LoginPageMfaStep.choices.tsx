import { KeyRoundIcon, MailIcon, ShieldCheckIcon } from 'lucide-react';
import { Button } from '@/components/ui/button';
import type { LoginPageMfaStepProps } from './LoginPageMfaStep.types';

export function LoginPageMfaMethodChoices({
  hasTotp,
  hasEmail,
  hasWebAuthn,
  hasRecovery,
  availableCount,
  onMfaMethodSelect,
}: Pick<
  LoginPageMfaStepProps,
  'hasTotp' | 'hasEmail' | 'hasWebAuthn' | 'hasRecovery' | 'availableCount' | 'onMfaMethodSelect'
>) {
  return (
    <div className="flex flex-col gap-2">
      {hasTotp && (
        <Button
          variant="outline"
          className="w-full justify-start gap-3"
          onClick={() => onMfaMethodSelect('totp')}
        >
          <ShieldCheckIcon className="size-4 text-muted-foreground" />
          Code d&apos;authentification (TOTP)
        </Button>
      )}
      {hasWebAuthn && (
        <Button
          variant="outline"
          className="w-full justify-start gap-3"
          onClick={() => onMfaMethodSelect('webauthn')}
        >
          <KeyRoundIcon className="size-4 text-muted-foreground" />
          Clé de sécurité (WebAuthn)
        </Button>
      )}
      {hasRecovery && (
        <Button
          variant="outline"
          className="w-full justify-start gap-3"
          onClick={() => onMfaMethodSelect('recovery')}
        >
          <MailIcon className="size-4 text-muted-foreground" />
          Code de récupération
        </Button>
      )}
      {hasEmail && (
        <Button
          variant="outline"
          className="w-full justify-start gap-3"
          onClick={() => onMfaMethodSelect('email')}
        >
          <MailIcon className="size-4 text-muted-foreground" />
          Code par email
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
