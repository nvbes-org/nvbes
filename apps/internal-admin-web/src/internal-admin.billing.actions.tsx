import { useMutation, useQueryClient } from '@tanstack/react-query';
import { AlertTriangle, RotateCcw, Save } from 'lucide-react';
import { useEffect, useState } from 'react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import {
  createCreditNote,
  createManualCompensation,
  createProviderMigration,
  createRefundIntent,
  createWriteOff,
  overrideGracePeriod,
  replayProviderEvent,
} from './internal-admin.api';
import { LockedState } from './internal-admin.locked-state';
import type { AdminCredentials, CreditNoteRequest } from './internal-admin.types';

type ActionKey =
  | 'credit-note'
  | 'write-off'
  | 'refund'
  | 'replay'
  | 'migration'
  | 'grace'
  | 'manual-comp';

const actionLabels: Record<ActionKey, string> = {
  'credit-note': 'Credit note',
  'write-off': 'Write-off',
  refund: 'Refund intent',
  replay: 'Replay provider event',
  migration: 'Provider migration',
  grace: 'Grace override',
  'manual-comp': 'Manual compensation',
};

const actionConfirmCodes: Record<ActionKey, string> = {
  'credit-note': 'CREATE CREDIT NOTE',
  'write-off': 'WRITE OFF',
  refund: 'CREATE REFUND',
  replay: 'REPLAY EVENT',
  migration: 'PLAN MIGRATION',
  grace: 'OVERRIDE GRACE',
  'manual-comp': 'CREATE COMPENSATION',
};

export function BillingActions({
  credentials,
  disabled,
  replayDraft,
}: {
  credentials: AdminCredentials;
  disabled: boolean;
  replayDraft: { nonce: number; provider: string; providerEventId: string } | null;
}) {
  const queryClient = useQueryClient();
  const [active, setActive] = useState<ActionKey>('credit-note');
  const [confirmCode, setConfirmCode] = useState('');
  const [result, setResult] = useState<string>('Aucune action executee.');
  const expectedCode = actionConfirmCodes[active];
  const mutation = useMutation({
    mutationFn: (form: FormData) => submitAction(credentials, active, confirmCode, form),
    onSuccess: async (data) => {
      setResult(JSON.stringify(data, null, 2));
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ['billing-overview', credentials.workspaceId] }),
        queryClient.invalidateQueries({
          queryKey: ['internal-audit-events', credentials.workspaceId],
        }),
        queryClient.invalidateQueries({
          queryKey: ['provider-event-failures', credentials.workspaceId],
        }),
        queryClient.invalidateQueries({ queryKey: ['billing-search', credentials.workspaceId] }),
      ]);
    },
    onError: (error) => setResult(error instanceof Error ? error.message : 'Action failed'),
  });
  const canSubmit = !disabled && confirmCode.trim() === expectedCode && !mutation.isPending;

  useEffect(() => {
    setConfirmCode('');
  }, [active]);

  useEffect(() => {
    if (!replayDraft) return;
    setActive('replay');
    setResult('Replay form prefilled from provider event failure.');
  }, [replayDraft]);

  return (
    <section className="border-border bg-card rounded-lg border p-4" id="mutations">
      <div className="mb-4 flex items-start gap-3">
        <div className="bg-destructive/10 text-destructive flex size-9 items-center justify-center rounded-md">
          <AlertTriangle className="size-4" />
        </div>
        <div>
          <h2 className="text-sm font-semibold">Actions sensibles</h2>
          <p className="text-muted-foreground text-xs">
            Chaque mutation exige un motif detaille pour l'audit.
          </p>
        </div>
      </div>

      <div className="mb-4 flex flex-wrap gap-2">
        {(Object.keys(actionLabels) as ActionKey[]).map((key) => (
          <Button
            key={key}
            onClick={() => setActive(key)}
            type="button"
            variant={active === key ? 'default' : 'outline'}
          >
            {actionLabels[key]}
          </Button>
        ))}
      </div>

      <form
        className="grid gap-4"
        onSubmit={(event) => {
          event.preventDefault();
          if (!disabled) mutation.mutate(new FormData(event.currentTarget));
        }}
      >
        {disabled ? (
          <LockedState label="Les mutations sont bloquees tant que le contexte operateur est incomplet." />
        ) : null}
        <ActionFields action={active} disabled={disabled} replayDraft={replayDraft} />
        <Field label="Motif audit">
          <Textarea
            disabled={disabled}
            name="reason"
            placeholder="Decrire le contexte, ticket, approbation, impact..."
            required
          />
        </Field>
        <div className="bg-muted/40 rounded-md border p-3">
          <div className="mb-2 flex flex-wrap items-center justify-between gap-2">
            <Label className="text-xs">Confirmation operateur</Label>
            <code className="bg-background rounded border px-2 py-1 font-mono text-xs">
              {expectedCode}
            </code>
          </div>
          <Input
            autoComplete="off"
            className="font-mono text-xs"
            disabled={disabled}
            onChange={(event) => setConfirmCode(event.target.value)}
            placeholder="Recopier le code de confirmation"
            value={confirmCode}
          />
          <p className="text-muted-foreground mt-2 text-xs">
            Le bouton Executer reste bloque tant que ce code ne correspond pas exactement.
          </p>
        </div>
        <div className="flex items-center gap-2">
          <Button disabled={!canSubmit} type="submit">
            <Save className="size-4" />
            Executer
          </Button>
          <Button
            onClick={() => setResult('Aucune action executee.')}
            type="button"
            variant="outline"
          >
            <RotateCcw className="size-4" />
            Reset
          </Button>
        </div>
      </form>

      <pre className="bg-muted text-muted-foreground mt-4 max-h-56 overflow-auto rounded-md p-3 text-xs">
        {result}
      </pre>
    </section>
  );
}

function ActionFields({
  action,
  disabled,
  replayDraft,
}: {
  action: ActionKey;
  disabled: boolean;
  replayDraft: { nonce: number; provider: string; providerEventId: string } | null;
}) {
  if (action === 'replay') {
    return (
      <div className="grid gap-3 md:grid-cols-2" key={replayDraft?.nonce ?? 'empty-replay'}>
        <Field label="Provider">
          <Input
            defaultValue={replayDraft?.provider}
            disabled={disabled}
            name="provider"
            placeholder="stripe"
            required
          />
        </Field>
        <Field label="Provider event ID">
          <Input
            defaultValue={replayDraft?.providerEventId}
            disabled={disabled}
            name="provider_event_id"
            required
          />
        </Field>
      </div>
    );
  }
  if (action === 'migration') {
    return (
      <div className="grid gap-3 md:grid-cols-2">
        <Field label="From provider">
          <Input disabled={disabled} name="from_provider" placeholder="stripe" required />
        </Field>
        <Field label="To provider">
          <Input disabled={disabled} name="to_provider" placeholder="mollie" required />
        </Field>
      </div>
    );
  }
  if (action === 'grace') {
    return (
      <div className="grid gap-3 md:grid-cols-2">
        <Field label="Subscription ID">
          <Input disabled={disabled} name="subscription_id" />
        </Field>
        <Field label="Grace days">
          <Input disabled={disabled} name="grace_days" required type="number" />
        </Field>
      </div>
    );
  }
  if (action === 'manual-comp') {
    return (
      <AmountFields disabled={disabled}>
        <Field label="Direction">
          <Input disabled={disabled} name="direction" placeholder="customer_credit" required />
        </Field>
      </AmountFields>
    );
  }
  if (action === 'refund') {
    return (
      <AmountFields disabled={disabled}>
        <Field label="Payment ID">
          <Input disabled={disabled} name="payment_id" required />
        </Field>
        <Field label="Provider">
          <Input disabled={disabled} name="provider" placeholder="stripe" required />
        </Field>
      </AmountFields>
    );
  }
  return (
    <AmountFields disabled={disabled}>
      <Field label="Invoice ID">
        <Input disabled={disabled} name="invoice_id" required />
      </Field>
    </AmountFields>
  );
}

function AmountFields({ children, disabled }: { children: React.ReactNode; disabled: boolean }) {
  return (
    <div className="grid gap-3 md:grid-cols-2">
      {children}
      <Field label="Amount minor">
        <Input disabled={disabled} name="amount_minor" required type="number" />
      </Field>
      <Field label="Currency">
        <Input defaultValue="EUR" disabled={disabled} name="currency" required />
      </Field>
    </div>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="grid gap-1.5">
      <Label className="text-xs">{label}</Label>
      {children}
    </div>
  );
}

async function submitAction(
  credentials: AdminCredentials,
  action: ActionKey,
  confirmCode: string,
  form: FormData,
) {
  const reason = text(form, 'reason');
  if (action === 'replay') {
    return replayProviderEvent(credentials, {
      confirm_code: confirmCode,
      provider: text(form, 'provider'),
      provider_event_id: text(form, 'provider_event_id'),
      reason,
    });
  }
  if (action === 'migration') {
    return createProviderMigration(credentials, {
      confirm_code: confirmCode,
      from_provider: text(form, 'from_provider'),
      to_provider: text(form, 'to_provider'),
      reason,
    });
  }
  if (action === 'grace') {
    return overrideGracePeriod(credentials, {
      confirm_code: confirmCode,
      subscription_id: optionalText(form, 'subscription_id'),
      grace_days: numberValue(form, 'grace_days'),
      reason,
    });
  }
  if (action === 'manual-comp') {
    return createManualCompensation(credentials, {
      confirm_code: confirmCode,
      amount_minor: numberValue(form, 'amount_minor'),
      currency: text(form, 'currency'),
      direction: text(form, 'direction'),
      reason,
    });
  }
  if (action === 'refund') {
    return createRefundIntent(credentials, {
      ...creditPayload(form, confirmCode),
      payment_id: text(form, 'payment_id'),
      provider: text(form, 'provider'),
      reason,
    });
  }
  return action === 'write-off'
    ? createWriteOff(credentials, creditPayload(form, confirmCode))
    : createCreditNote(credentials, creditPayload(form, confirmCode));
}

function creditPayload(form: FormData, confirmCode: string): CreditNoteRequest {
  return {
    confirm_code: confirmCode,
    invoice_id: text(form, 'invoice_id'),
    amount_minor: numberValue(form, 'amount_minor'),
    currency: text(form, 'currency'),
    reason: text(form, 'reason'),
  };
}

function text(form: FormData, key: string): string {
  const value = form.get(key);
  return typeof value === 'string' ? value.trim() : '';
}

function optionalText(form: FormData, key: string): string | undefined {
  const value = text(form, key);
  return value.length > 0 ? value : undefined;
}

function numberValue(form: FormData, key: string): number {
  return Number(text(form, key));
}
