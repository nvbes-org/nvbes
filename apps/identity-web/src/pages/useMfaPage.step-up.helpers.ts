import {
  completeWebAuthnStepUp,
  MfaError,
  removeMfaFactor,
  stepUp,
  WebauthnBrowserError,
} from '@nvbes/identity-sdk-web';
import type { MfaFactorView } from '@nvbes/identity-sdk-core/src/types';
import type { StepUpMethod } from './MfaPage.shared';
import { WEBAUTHN_TIMEOUT_MS } from './MfaPage.shared';

export type SetFactors = React.Dispatch<React.SetStateAction<MfaFactorView[]>>;

export async function removeFactorAfterStepUp(removingId: string, setFactors: SetFactors) {
  await removeMfaFactor('', removingId);
  setFactors((prev) => prev.filter((factor) => factor.id !== removingId));
}

export async function runStepUpVerification({
  stepUpMethod,
  stepUpPassword,
  stepUpTotpCode,
  stepUpRecoveryCode,
}: {
  stepUpMethod: StepUpMethod | null;
  stepUpPassword: string;
  stepUpTotpCode: string;
  stepUpRecoveryCode: string;
}) {
  if (stepUpMethod === 'totp') {
    await stepUp('', { totpCode: stepUpTotpCode });
    return;
  }

  if (stepUpMethod === 'webauthn') {
    await completeWebAuthnStepUp('', undefined, {
      timeoutMs: WEBAUTHN_TIMEOUT_MS,
    });
    return;
  }

  if (stepUpMethod === 'recovery') {
    await stepUp('', { recoveryCode: stepUpRecoveryCode });
    return;
  }

  await stepUp('', { password: stepUpPassword });
}

export function getStepUpErrorMessage(err: unknown) {
  if (err instanceof MfaError) {
    return err.message;
  }

  if (err instanceof WebauthnBrowserError && err.code === 'webauthn_timeout') {
    return 'La demande WebAuthn a expiré. Veuillez réessayer.';
  }

  return err instanceof Error ? err.message : 'Échec de la vérification';
}
