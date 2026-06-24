import { useMutation, useQueryClient } from '@tanstack/react-query';
import { AlertTriangle, CheckCircle2, FileLock2, ReceiptText, RotateCcw } from 'lucide-react';
import type { ReactNode } from 'react';
import { useState } from 'react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import {
  closeDunningCase,
  holdRevenueInvoice,
  releaseRevenueInvoice,
  reopenDunningCase,
  resolveRevenueDispute,
  reviewRevenueDispute,
} from './internal-admin.api';
import type { AdminCredentials, RevenueActionResult } from './internal-admin.types';

type RevenueActionKind =
  | 'closeDunning'
  | 'holdInvoice'
  | 'releaseInvoice'
  | 'reopenDunning'
  | 'resolveDispute'
  | 'reviewDispute';

const confirmCodes: Record<RevenueActionKind, string> = {
  closeDunning: 'CLOSE DUNNING CASE',
  reopenDunning: 'REOPEN DUNNING CASE',
  holdInvoice: 'HOLD INVOICE',
  releaseInvoice: 'RELEASE INVOICE',
  reviewDispute: 'REVIEW DISPUTE',
  resolveDispute: 'RESOLVE DISPUTE',
};

export function RevenueActionsPanel({
  credentials,
  disabled,
}: {
  credentials: AdminCredentials;
  disabled: boolean;
}) {
  const queryClient = useQueryClient();
  const [action, setAction] = useState<RevenueActionKind>('holdInvoice');
  const [targetId, setTargetId] = useState('');
  const [confirmCode, setConfirmCode] = useState('');
  const [reason, setReason] = useState('');
  const [result, setResult] = useState<RevenueActionResult | null>(null);

  const mutation = useMutation({
    mutationFn: () =>
      executeRevenueAction(credentials, action, {
        confirmCode,
        reason,
        targetId,
      }),
    onSuccess: async (payload) => {
      setResult(payload);
      await queryClient.invalidateQueries({ queryKey: ['revenue-center'] });
    },
  });

  return (
    <div className="mt-4 rounded-md border">
      <div className="flex flex-col gap-2 border-b p-3 md:flex-row md:items-center md:justify-between">
        <div>
          <h3 className="text-sm font-medium">Actions revenue</h3>
          <p className="text-muted-foreground text-xs">
            Recouvrement, hold invoice et workflow dispute avec audit.
          </p>
        </div>
        <div className="flex flex-wrap gap-2">
          <ActionButton
            action="holdInvoice"
            current={action}
            label="Hold invoice"
            onSelect={setAction}
          />
          <ActionButton
            action="releaseInvoice"
            current={action}
            label="Release invoice"
            onSelect={setAction}
          />
          <ActionButton
            action="closeDunning"
            current={action}
            label="Close dunning"
            onSelect={setAction}
          />
          <ActionButton
            action="reopenDunning"
            current={action}
            label="Reopen dunning"
            onSelect={setAction}
          />
          <ActionButton
            action="reviewDispute"
            current={action}
            label="Review dispute"
            onSelect={setAction}
          />
          <ActionButton
            action="resolveDispute"
            current={action}
            label="Resolve dispute"
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
            placeholder={confirmCodes[action]}
            value={confirmCode}
          />
        </Field>
        <div className="md:col-span-2 xl:col-span-4">
          <Field label="Reason">
            <Textarea
              disabled={disabled || mutation.isPending}
              onChange={(event) => setReason(event.target.value)}
              placeholder="ticket REV-123 approved"
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

type RevenueActionPayload = {
  confirmCode: string;
  reason: string;
  targetId: string;
};

function executeRevenueAction(
  credentials: AdminCredentials,
  action: RevenueActionKind,
  payload: RevenueActionPayload,
) {
  const body = {
    confirm_code: payload.confirmCode,
    reason: payload.reason,
  };
  if (action === 'holdInvoice') return holdRevenueInvoice(credentials, payload.targetId, body);
  if (action === 'releaseInvoice') {
    return releaseRevenueInvoice(credentials, payload.targetId, body);
  }
  if (action === 'closeDunning') return closeDunningCase(credentials, payload.targetId, body);
  if (action === 'reopenDunning') return reopenDunningCase(credentials, payload.targetId, body);
  if (action === 'reviewDispute') return reviewRevenueDispute(credentials, payload.targetId, body);
  return resolveRevenueDispute(credentials, payload.targetId, body);
}

function ActionButton({
  action,
  current,
  label,
  onSelect,
}: {
  action: RevenueActionKind;
  current: RevenueActionKind;
  label: string;
  onSelect: (action: RevenueActionKind) => void;
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

function ActionIcon({ action }: { action: RevenueActionKind }) {
  if (action === 'holdInvoice') return <FileLock2 className="size-4" />;
  if (action === 'releaseInvoice') return <ReceiptText className="size-4" />;
  if (action === 'reopenDunning') return <RotateCcw className="size-4" />;
  if (action === 'resolveDispute') return <CheckCircle2 className="size-4" />;
  return <AlertTriangle className="size-4" />;
}

function targetLabel(action: RevenueActionKind): string {
  if (action === 'holdInvoice' || action === 'releaseInvoice') return 'Invoice ID';
  if (action === 'closeDunning' || action === 'reopenDunning') return 'Dunning case ID';
  return 'Dispute ID';
}

function Field({ children, label }: { children: ReactNode; label: string }) {
  return (
    <Label className="flex flex-col items-start gap-1 text-xs">
      {label}
      {children}
    </Label>
  );
}
