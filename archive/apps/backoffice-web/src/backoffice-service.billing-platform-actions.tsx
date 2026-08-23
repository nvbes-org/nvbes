import { useMutation, useQueryClient } from '@tanstack/react-query';
import { FileText, Route, ShieldCheck, XCircle } from 'lucide-react';
import type { ReactNode } from 'react';
import { useState } from 'react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import {
  activateEinvoicingProfile,
  approveKycProfile,
  disableProviderRoutingRule,
  enableProviderRoutingRule,
  rejectKycProfile,
} from './backoffice-service.api';
import { strongConfirmationCode } from './backoffice-service.strong-confirmation';
import type { AdminCredentials, BillingPlatformActionResult } from './backoffice-service.types';

type BillingPlatformActionKind =
  | 'activateEinvoicing'
  | 'approveKyc'
  | 'disableRouting'
  | 'enableRouting'
  | 'rejectKyc';

const confirmCodes: Record<BillingPlatformActionKind, string> = {
  enableRouting: 'ENABLE ROUTING RULE',
  disableRouting: 'DISABLE ROUTING RULE',
  approveKyc: 'APPROVE KYC',
  rejectKyc: 'REJECT KYC',
  activateEinvoicing: 'ACTIVATE EINVOICING',
};

export function BillingPlatformActionsPanel({
  credentials,
  disabled,
}: {
  credentials: AdminCredentials;
  disabled: boolean;
}) {
  const queryClient = useQueryClient();
  const [action, setAction] = useState<BillingPlatformActionKind>('approveKyc');
  const [targetId, setTargetId] = useState('');
  const [confirmCode, setConfirmCode] = useState('');
  const [reason, setReason] = useState('');
  const [result, setResult] = useState<BillingPlatformActionResult | null>(null);
  const requiredConfirmCode = billingPlatformConfirmCode(action, targetId);
  const isReasonReady = reason.trim().length >= 12;
  const isConfirmationReady = confirmCode.trim() === requiredConfirmCode;

  const mutation = useMutation({
    mutationFn: () =>
      executeBillingPlatformAction(credentials, action, {
        confirmCode,
        reason,
        targetId,
      }),
    onSuccess: async (payload) => {
      setResult(payload);
      await queryClient.invalidateQueries({ queryKey: ['billing-platform-center'] });
    },
  });

  return (
    <div className="mt-4 rounded-md border">
      <div className="flex flex-col gap-2 border-b p-3 md:flex-row md:items-center md:justify-between">
        <div>
          <h3 className="text-sm font-medium">Actions billing platform</h3>
          <p className="text-muted-foreground text-xs">
            Routing provider, review KYC et activation e-invoicing avec audit.
          </p>
        </div>
        <div className="flex flex-wrap gap-2">
          <ActionButton
            action="enableRouting"
            current={action}
            label="Enable routing"
            onSelect={setAction}
          />
          <ActionButton
            action="disableRouting"
            current={action}
            label="Disable routing"
            onSelect={setAction}
          />
          <ActionButton
            action="approveKyc"
            current={action}
            label="Approve KYC"
            onSelect={setAction}
          />
          <ActionButton
            action="rejectKyc"
            current={action}
            label="Reject KYC"
            onSelect={setAction}
          />
          <ActionButton
            action="activateEinvoicing"
            current={action}
            label="Activate e-invoicing"
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
            placeholder={requiredConfirmCode}
            value={confirmCode}
          />
        </Field>
        <div className="md:col-span-2 xl:col-span-4">
          <Field label="Reason">
            <Textarea
              disabled={disabled || mutation.isPending}
              onChange={(event) => setReason(event.target.value)}
              placeholder="ticket BPL-123 approved"
              value={reason}
            />
          </Field>
        </div>
      </div>
      <div className="flex flex-col gap-2 border-t p-3 md:flex-row md:items-center md:justify-between">
        <div className="text-muted-foreground text-xs">
          Code requis: <span className="text-foreground font-medium">{requiredConfirmCode}</span>
        </div>
        <Button
          disabled={disabled || !isReasonReady || !isConfirmationReady || mutation.isPending}
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

function billingPlatformConfirmCode(action: BillingPlatformActionKind, targetId: string): string {
  return strongConfirmationCode(confirmCodes[action], targetId);
}

type BillingPlatformActionPayload = {
  confirmCode: string;
  reason: string;
  targetId: string;
};

function executeBillingPlatformAction(
  credentials: AdminCredentials,
  action: BillingPlatformActionKind,
  payload: BillingPlatformActionPayload,
) {
  const body = {
    confirm_code: payload.confirmCode,
    reason: payload.reason,
  };
  if (action === 'enableRouting')
    return enableProviderRoutingRule(credentials, payload.targetId, body);
  if (action === 'disableRouting')
    return disableProviderRoutingRule(credentials, payload.targetId, body);
  if (action === 'approveKyc') return approveKycProfile(credentials, payload.targetId, body);
  if (action === 'rejectKyc') return rejectKycProfile(credentials, payload.targetId, body);
  return activateEinvoicingProfile(credentials, payload.targetId, body);
}

function ActionButton({
  action,
  current,
  label,
  onSelect,
}: {
  action: BillingPlatformActionKind;
  current: BillingPlatformActionKind;
  label: string;
  onSelect: (action: BillingPlatformActionKind) => void;
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

function ActionIcon({ action }: { action: BillingPlatformActionKind }) {
  if (action === 'enableRouting') return <Route className="size-4" />;
  if (action === 'disableRouting') return <XCircle className="size-4" />;
  if (action === 'approveKyc') return <ShieldCheck className="size-4" />;
  if (action === 'rejectKyc') return <XCircle className="size-4" />;
  return <FileText className="size-4" />;
}

function targetLabel(action: BillingPlatformActionKind): string {
  if (action === 'enableRouting' || action === 'disableRouting') return 'Routing rule ID';
  if (action === 'approveKyc' || action === 'rejectKyc') return 'KYC profile ID';
  return 'E-invoicing profile ID';
}

function Field({ children, label }: { children: ReactNode; label: string }) {
  return (
    <Label className="flex flex-col items-start gap-1 text-xs">
      {label}
      {children}
    </Label>
  );
}
