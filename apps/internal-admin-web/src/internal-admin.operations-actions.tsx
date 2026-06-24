import { useMutation, useQueryClient } from '@tanstack/react-query';
import { CheckCircle2, FileDown, RefreshCcw } from 'lucide-react';
import type { ReactNode } from 'react';
import { useState } from 'react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import {
  replayExportRun,
  replayOperationsProviderEvent,
  resolveReconciliationDifference,
} from './internal-admin.api';
import { strongConfirmationCode } from './internal-admin.strong-confirmation';
import type { AdminCredentials, OperationsActionResult } from './internal-admin.types';

type OperationsActionKind = 'replayExport' | 'replayProvider' | 'resolveRecon';

const confirmCodes: Record<OperationsActionKind, string> = {
  replayProvider: 'REPLAY PROVIDER EVENT',
  replayExport: 'REPLAY EXPORT RUN',
  resolveRecon: 'RESOLVE RECON DIFFERENCE',
};

export function OperationsActionsPanel({
  credentials,
  disabled,
}: {
  credentials: AdminCredentials;
  disabled: boolean;
}) {
  const queryClient = useQueryClient();
  const [action, setAction] = useState<OperationsActionKind>('replayProvider');
  const [targetId, setTargetId] = useState('');
  const [confirmCode, setConfirmCode] = useState('');
  const [reason, setReason] = useState('');
  const [result, setResult] = useState<OperationsActionResult | null>(null);
  const expectedConfirmCode = strongConfirmationCode(confirmCodes[action], targetId);

  const mutation = useMutation({
    mutationFn: () =>
      executeOperationsAction(credentials, action, {
        confirmCode,
        reason,
        targetId,
      }),
    onSuccess: async (payload) => {
      setResult(payload);
      await queryClient.invalidateQueries({ queryKey: ['operations-center'] });
    },
  });

  return (
    <div className="mt-4 rounded-md border">
      <div className="flex flex-col gap-2 border-b p-3 md:flex-row md:items-center md:justify-between">
        <div>
          <h3 className="text-sm font-medium">Actions operations</h3>
          <p className="text-muted-foreground text-xs">
            Replay de jobs critiques et resolution des ecarts avec audit.
          </p>
        </div>
        <div className="flex flex-wrap gap-2">
          <ActionButton
            action="replayProvider"
            current={action}
            label="Provider event"
            onSelect={setAction}
          />
          <ActionButton
            action="replayExport"
            current={action}
            label="Export run"
            onSelect={setAction}
          />
          <ActionButton
            action="resolveRecon"
            current={action}
            label="Recon diff"
            onSelect={setAction}
          />
        </div>
      </div>
      <div className="grid gap-3 p-3 md:grid-cols-2 xl:grid-cols-4">
        <Field label={targetLabel(action)}>
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
            placeholder={expectedConfirmCode}
            value={confirmCode}
          />
        </Field>
        <div className="md:col-span-2 xl:col-span-4">
          <Field label="Reason">
            <Textarea
              disabled={disabled || mutation.isPending}
              onChange={(event) => setReason(event.target.value)}
              placeholder="ticket OPS-123 approved"
              value={reason}
            />
          </Field>
        </div>
      </div>
      <div className="flex flex-col gap-2 border-t p-3 md:flex-row md:items-center md:justify-between">
        <div className="text-muted-foreground text-xs">
          Code requis: <span className="text-foreground font-medium">{expectedConfirmCode}</span>
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

type OperationsActionPayload = {
  confirmCode: string;
  reason: string;
  targetId: string;
};

function executeOperationsAction(
  credentials: AdminCredentials,
  action: OperationsActionKind,
  payload: OperationsActionPayload,
) {
  const body = {
    confirm_code: payload.confirmCode,
    reason: payload.reason,
  };
  if (action === 'replayProvider') {
    return replayOperationsProviderEvent(credentials, payload.targetId, body);
  }
  if (action === 'replayExport') return replayExportRun(credentials, payload.targetId, body);
  return resolveReconciliationDifference(credentials, payload.targetId, body);
}

function ActionButton({
  action,
  current,
  label,
  onSelect,
}: {
  action: OperationsActionKind;
  current: OperationsActionKind;
  label: string;
  onSelect: (action: OperationsActionKind) => void;
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

function ActionIcon({ action }: { action: OperationsActionKind }) {
  if (action === 'replayExport') return <FileDown className="size-4" />;
  if (action === 'resolveRecon') return <CheckCircle2 className="size-4" />;
  return <RefreshCcw className="size-4" />;
}

function targetLabel(action: OperationsActionKind): string {
  if (action === 'replayProvider') return 'Provider event ID';
  if (action === 'replayExport') return 'Export run ID';
  return 'Reconciliation difference ID';
}

function Field({ children, label }: { children: ReactNode; label: string }) {
  return (
    <Label className="flex flex-col items-start gap-1 text-xs">
      {label}
      {children}
    </Label>
  );
}
