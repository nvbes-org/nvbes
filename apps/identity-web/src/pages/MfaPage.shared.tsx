import type { MfaFactorView } from '@nvbes/identity-sdk-core/src/types';
import { Fingerprint, KeyRound, Mail, ShieldAlert, TimerReset } from 'lucide-react';

export const FACTOR_ICONS: Record<string, React.ComponentType<{ className?: string }>> = {
  totp: TimerReset,
  email: Mail,
  webauthn: KeyRound,
  passkey: Fingerprint,
  recovery_code: ShieldAlert,
  recovery: ShieldAlert,
};

export const FACTOR_ADD_ACTIONS = [
  {
    icon: TimerReset,
    label: "Code d'authentification (TOTP)",
    path: '/account/mfa/totp/setup',
  },
  {
    icon: Fingerprint,
    label: 'Biométrie',
    path: '/account/mfa/passkey/setup',
  },
  {
    icon: KeyRound,
    label: 'Clé de sécurité',
    path: '/account/mfa/security-key/setup',
  },
  {
    icon: ShieldAlert,
    label: 'Codes de récupération',
    path: '/account/mfa/recovery-codes',
  },
] as const;

export type StepUpMethod = 'password' | 'totp' | 'webauthn' | 'recovery';

export const WEBAUTHN_TIMEOUT_MS = 60_000;

export function getFactorTypeIcon(type: string): React.ComponentType<{ className?: string }> {
  return FACTOR_ICONS[type] ?? TimerReset;
}

export function factorLabel(factor: MfaFactorView) {
  if (factor.factor_type === 'webauthn' && factor.kind === 'passkey') {
    return 'Biométrie';
  }
  if (factor.factor_type === 'webauthn' && factor.kind === 'security_key') {
    return 'Clé de sécurité';
  }

  switch (factor.factor_type) {
    case 'totp':
      return "Code d'authentification (TOTP)";
    case 'email':
      return 'Code par email';
    case 'webauthn':
      return 'Clé de sécurité (WebAuthn)';
    case 'recovery_code':
    case 'recovery':
      return 'Codes de récupération';
    default:
      return factor.factor_type;
  }
}
