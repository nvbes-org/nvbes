import { KeyRoundIcon, ShieldAlertIcon, ShieldCheckIcon } from 'lucide-react';
import { MfaMethodChoiceList, type MfaMethodChoice } from '@/components/MfaMethodChoiceList';
import type { MfaMethod } from './LoginPage.mfa';
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
  const choices: MfaMethodChoice<MfaMethod>[] = [];
  if (hasWebAuthn) {
    choices.push({
      value: 'webauthn',
      label: 'Utiliser une passkey ou une clé de sécurité',
      icon: KeyRoundIcon,
      variant: 'default',
    });
  }
  if (hasTotp) {
    choices.push({
      value: 'totp',
      label: 'Utiliser le code TOTP de secours',
      icon: ShieldCheckIcon,
    });
  }
  if (hasRecovery) {
    choices.push({
      value: 'recovery',
      label: 'Code de récupération',
      icon: ShieldAlertIcon,
    });
  }

  return (
    <MfaMethodChoiceList
      choices={choices}
      buttonClassName="w-full"
      emptyMessage={
        availableCount === 0 ? "Aucun facteur MFA n'a été proposé pour cette session." : undefined
      }
      onSelect={onMfaMethodSelect}
    />
  );
}
