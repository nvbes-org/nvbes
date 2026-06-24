import { useMutation, useQueryClient } from '@tanstack/react-query';
import { CheckCircle2, ShieldAlert, XCircle } from 'lucide-react';
import type { ReactNode } from 'react';
import { useState } from 'react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import { approveRiskPolicy, blockRiskPolicy, resolveRiskSignal } from './internal-admin.api';
import type { AdminCredentials, RiskActionResult } from './internal-admin.types';

type RiskActionKind = 'approvePolicy' | 'blockPolicy' | 'resolveSignal';

const confirmCodes: Record<RiskActionKind, string> = {
  approvePolicy: 'APPROVE RISK POLICY',
  blockPolicy: 'BLOCK RISK POLICY',
  resolveSignal: 'RESOLVE RISK SIGNAL',
};

export function RiskDecisionActionsPanel({
  credentials,
  disabled,
}: {
  credentials: AdminCredentials;
  disabled: boolean;
}) {
  const queryClient = useQueryClient();
  const [action, setAction] = useState<RiskActionKind>('resolveSignal');
  const [targetId, setTargetId] = useState('');
  const [confirmCode, setConfirmCode] = useState('');
  const [reason, setReason] = useState('');
  const [result, setResult] = useState<RiskActionResult | null>(null);

  const mutation = useMutation({
    mutationFn: () =>
      executeRiskAction(credentials, action, {
        confirmCode,
        reason,
        targetId,
      }),
    onSuccess: async (payload) => {
      setResult(payload);
      await queryClient.invalidateQueries({ queryKey: ['risk-decision-center'] });
    },
  });

  return (
    <div className="mt-4 rounded-md border">
      <div className="flex flex-col gap-2 border-b p-3 md:flex-row md:items-center md:justify-between">
        <div>
          <h3 className="text-sm font-medium">Actions risk decision</h3>
          <p className="text-muted-foreground text-xs">
            Approbation, blocage de policy et resolution de signal avec audit.
          </p>
        </div>
        <div className="flex flex-wrap gap-2">
          <ActionButton
            action="resolveSignal"
            current={action}
            label="Resolve signal"
            onSelect={setAction}
          />
          <ActionButton
            action="approvePolicy"
            current={action}
            label="Approve policy"
            onSelect={setAction}
          />
          <ActionButton
            action="blockPolicy"
            current={action}
            label="Block policy"
            onSelect={setAction}
          />
        </div>
      </div>
      <div className="grid gap-3 p-3 md:grid-cols-2 xl:grid-cols-4">
        <Field label={action === 'resolveSignal' ? 'Risk signal ID' : 'Policy snapshot ID'}>
          <Input
            disabled={disabled || mutation.isPending}
            onChange={(event) => setTargetId(event.target.value)}
            value={targetId}
          />
        </Field>
        <Field label="Confirmation code">
          <Input
            disabled={disabled || mutation.isPending}
            onChange={(event) => setConfirmCode(event.target.value)}
            placeholder={confirmCodes[action]}
            value={confirmCode}
          />
        </Field>
        <div className="md:col-span-2 xl:col-span-4">
          <Field label="Reason">
            <Textarea
              disabled={disabled || mutation.isPending}
              onChange={(event) => setReason(event.target.value)}
              placeholder="ticket RISK-123 reviewed"
              value={reason}
            />
          </Field>
        </div>
      </div>
      <div className="flex flex-col gap-2 border-t p-3 md:flex-row md:items-center md:justify-between">
        <div className="text-muted-foreground text-xs">
          Code requis: <span className="text-foreground font-medium">{confirmCodes[action]}</span>
        </div>
        <Button
          disabled={disabled || mutation.isPending}
          onClick={() => mutation.mutate()}
          type="button"
        >
          <ActionIcon action={action} />
          Execute action
        </Button>
      </div>
      {mutation.error ? (
        <p className="text-destructive border-t p-3 text-sm">{mutation.error.message}</p>
      ) : null}
      {result ? (
        <p className="text-muted-foreground border-t p-3 text-sm">
          {result.audit_action} - {result.status} - {result.object_id}
        </p>
      ) : null}
    </div>
  );
}

type RiskActionPayload = {
  confirmCode: string;
  reason: string;
  targetId: string;
};

function executeRiskAction(
  credentials: AdminCredentials,
  action: RiskActionKind,
  payload: RiskActionPayload,
) {
  const body = {
    confirm_code: payload.confirmCode,
    reason: payload.reason,
  };
  if (action === 'approvePolicy') {
    return approveRiskPolicy(credentials, payload.targetId, body);
  }
  if (action === 'blockPolicy') {
    return blockRiskPolicy(credentials, payload.targetId, body);
  }
  return resolveRiskSignal(credentials, payload.targetId, body);
}

function ActionButton({
  action,
  current,
  label,
  onSelect,
}: {
  action: RiskActionKind;
  current: RiskActionKind;
  label: string;
  onSelect: (action: RiskActionKind) => void;
}) {
  return (
    <Button
      onClick={() => onSelect(action)}
      size="sm"
      type="button"
      variant={current === action ? 'default' : 'outline'}
    >
      <ActionIcon action={action} />
      {label}
    </Button>
  );
}

function ActionIcon({ action }: { action: RiskActionKind }) {
  if (action === 'approvePolicy') return <CheckCircle2 className="size-4" />;
  if (action === 'blockPolicy') return <XCircle className="size-4" />;
  return <ShieldAlert className="size-4" />;
}

function Field({ children, label }: { children: ReactNode; label: string }) {
  return (
    <Label className="flex flex-col items-start gap-1 text-xs">
      {label}
      {children}
    </Label>
  );
}
