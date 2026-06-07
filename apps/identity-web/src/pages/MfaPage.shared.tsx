import type { MfaFactorView } from '@nvbes/identity-sdk-core/src/types';
import { Fingerprint, KeyRound, ShieldAlert, Smartphone } from 'lucide-react';

export const FACTOR_ICONS: Record<string, React.ComponentType<{ className?: string }>> = {
  totp: Smartphone,
  webauthn: KeyRound,
  recovery: ShieldAlert,
};

export const FACTOR_ADD_ACTIONS = [
  {
    icon: Smartphone,
    label: "Code d'authentification (TOTP)",
    path: '/account/mfa/totp/setup',
  },
  {
    icon: Fingerprint,
    label: 'Passkey',
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
  return FACTOR_ICONS[type] ?? Smartphone;
}

export function factorLabel(factor: MfaFactorView) {
  if (factor.factor_type === 'webauthn' && factor.kind === 'passkey') {
    return 'Passkey';
  }
  if (factor.factor_type === 'webauthn' && factor.kind === 'security_key') {
    return 'Clé de sécurité';
  }

  switch (factor.factor_type) {
    case 'totp':
      return "Code d'authentification (TOTP)";
    case 'webauthn':
      return 'Clé de sécurité (WebAuthn)';
    case 'recovery':
      return 'Codes de récupération';
    default:
      return factor.factor_type;
  }
}
