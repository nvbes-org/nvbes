import { Fingerprint, KeyRound, ShieldAlert, TimerReset } from 'lucide-react';
import { MfaMethodChoiceList, type MfaMethodChoice } from '@/components/MfaMethodChoiceList';
import type { StepUpMethod } from './MfaPage.shared';
import type { MfaPageStepUpDialogProps } from './MfaPageStepUpDialog.types';

export function MfaPageStepUpMethodChoices({
  hasTotp,
  hasWebAuthn,
  hasRecovery,
  canUsePassword,
  onMethodSelect,
}: Pick<
  MfaPageStepUpDialogProps,
  'hasTotp' | 'hasWebAuthn' | 'hasRecovery' | 'canUsePassword' | 'onMethodSelect'
>) {
  const choices: MfaMethodChoice<StepUpMethod>[] = [];
  if (canUsePassword) {
    choices.push({ value: 'password', label: 'Mot de passe', icon: KeyRound });
  }
  if (hasTotp) {
    choices.push({
      value: 'totp',
      label: "Code d'authentification (TOTP)",
      icon: TimerReset,
    });
  }
  if (hasWebAuthn) {
    choices.push({
      value: 'webauthn',
      label: 'Biométrie ou clé de sécurité',
      icon: Fingerprint,
    });
  }
  if (hasRecovery) {
    choices.push({ value: 'recovery', label: 'Code de récupération', icon: ShieldAlert });
  }

  return <MfaMethodChoiceList choices={choices} onSelect={onMethodSelect} />;
}
