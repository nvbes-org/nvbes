import { HttpError } from '@nvbes/http-client';
import { completeWebAuthnStepUp, stepUp } from '@nvbes/identity-sdk-web';
import { type SubmitEvent, useCallback, useState } from 'react';
import { enterpriseClient } from '../enterprise.api';
import { AdminElevationDialog, type StepUpMethod } from './UsersPage.admin-elevation.dialog';

type DeferredElevation = {
  resolve: () => void;
  reject: (error: Error) => void;
  procedure?: AdminElevationProcedure;
};

export type AdminElevationProcedure = {
  reason: string;
  procedure_reference: string;
};

export function isAdminElevationCancelled(error: unknown): boolean {
  return error instanceof Error && error.message === 'admin_elevation_cancelled';
}

export function useAdminElevation({
  active,
  breakGlass,
  onGranted,
}: {
  active: boolean;
  breakGlass?: { procedure_reference: string } | null;
  onGranted: () => Promise<void>;
}) {
  const [deferred, setDeferred] = useState<DeferredElevation | null>(null);
  const [method, setMethod] = useState<StepUpMethod>('password');
  const [password, setPassword] = useState('');
  const [totpCode, setTotpCode] = useState('');
  const [recoveryCode, setRecoveryCode] = useState('');
  const [breakGlassReason, setBreakGlassReason] = useState('');
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const requestElevation = useCallback(
    async (force = false, procedure?: AdminElevationProcedure) => {
      if (active && !force) {
        return;
      }

      await new Promise<void>((resolve, reject) => {
        setError(null);
        setDeferred({ resolve, reject, procedure });
      });
    },
    [active],
  );

  const runElevated = useCallback(
    async <T,>(action: () => Promise<T>, procedure?: AdminElevationProcedure): Promise<T> => {
      await requestElevation(false, procedure);
      try {
        return await action();
      } catch (caught) {
        if (!isStepUpRequired(caught)) {
          throw caught;
        }
        await requestElevation(true, procedure);
        return action();
      }
    },
    [requestElevation],
  );

  async function submit(event: SubmitEvent<HTMLFormElement>) {
    event.preventDefault();
    setPending(true);
    setError(null);
    try {
      const procedure =
        deferred?.procedure ?? buildBreakGlassProcedure(breakGlass, breakGlassReason);
      if (method === 'webauthn') {
        await completeWebAuthnStepUp('');
      } else if (method === 'totp') {
        await stepUp('', { totpCode });
      } else if (method === 'recovery') {
        await stepUp('', { recoveryCode });
      } else {
        await stepUp('', { password });
      }
      await enterpriseClient.grantEnterpriseAdminElevation({
        duration_minutes: 15,
        reason: procedure?.reason,
        procedure_reference: procedure?.procedure_reference,
      });
      await onGranted();
      resetSecrets();
      deferred?.resolve();
      setDeferred(null);
    } catch (caught) {
      setError(readErrorMessage(caught));
    } finally {
      setPending(false);
    }
  }

  function cancel(open: boolean) {
    if (open) {
      return;
    }
    deferred?.reject(new Error('admin_elevation_cancelled'));
    setDeferred(null);
    setError(null);
    resetSecrets();
  }

  function resetSecrets() {
    setPassword('');
    setTotpCode('');
    setRecoveryCode('');
    setBreakGlassReason('');
  }

  return {
    dialog: (
      <AdminElevationDialog
        open={Boolean(deferred)}
        method={method}
        password={password}
        totpCode={totpCode}
        recoveryCode={recoveryCode}
        breakGlassActive={Boolean(breakGlass && !deferred?.procedure)}
        breakGlassProcedureReference={breakGlass?.procedure_reference ?? null}
        breakGlassReason={breakGlassReason}
        pending={pending}
        error={error}
        onOpenChange={cancel}
        onMethodChange={setMethod}
        onPasswordChange={setPassword}
        onTotpCodeChange={setTotpCode}
        onRecoveryCodeChange={setRecoveryCode}
        onBreakGlassReasonChange={setBreakGlassReason}
        onSubmit={submit}
      />
    ),
    pending,
    runElevated,
  };
}

function buildBreakGlassProcedure(
  breakGlass: { procedure_reference: string } | null | undefined,
  reason: string,
): AdminElevationProcedure | undefined {
  if (!breakGlass) {
    return undefined;
  }
  const trimmedReason = reason.trim();
  if (trimmedReason.length === 0) {
    throw new Error('Emergency procedure reason is required.');
  }
  return {
    procedure_reference: breakGlass.procedure_reference,
    reason: trimmedReason,
  };
}

function isStepUpRequired(error: unknown): boolean {
  if (error instanceof HttpError) {
    return readErrorCode(error.body) === 'step_up_required';
  }
  return error instanceof Error && error.message.includes('step_up_required');
}

function readErrorMessage(error: unknown): string {
  if (error instanceof HttpError) {
    return readEnvelopeMessage(error.body) ?? error.message;
  }
  return error instanceof Error ? error.message : 'Verification failed.';
}

function readEnvelopeMessage(body: unknown): string | null {
  if (!isErrorEnvelope(body)) {
    return null;
  }
  return body.error.message ?? body.error.code ?? null;
}

function readErrorCode(body: unknown): string | null {
  if (!isErrorEnvelope(body)) {
    return null;
  }
  return body.error.code ?? null;
}

function isErrorEnvelope(body: unknown): body is { error: { code?: string; message?: string } } {
  return (
    typeof body === 'object' &&
    body !== null &&
    'error' in body &&
    typeof (body as { error?: unknown }).error === 'object' &&
    (body as { error?: unknown }).error !== null
  );
}
