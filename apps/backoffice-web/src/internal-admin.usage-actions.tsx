import { useMutation, useQueryClient } from '@tanstack/react-query';
import { Ban, PencilLine, RotateCcw } from 'lucide-react';
import type { ReactNode } from 'react';
import { useState } from 'react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import { correctUsage, freezeUsageMeter, replayUsageRollup } from './internal-admin.api';
import { strongConfirmationCode } from './internal-admin.strong-confirmation';
import type { AdminCredentials, UsageActionResult } from './internal-admin.types';

type UsageActionKind = 'correction' | 'freeze' | 'replay';

const confirmCodes: Record<UsageActionKind, string> = {
  correction: 'CORRECT USAGE',
  freeze: 'FREEZE METER',
  replay: 'REPLAY ROLLUP',
};

export function UsageActionsPanel({
  credentials,
  disabled,
}: {
  credentials: AdminCredentials;
  disabled: boolean;
}) {
  const queryClient = useQueryClient();
  const [action, setAction] = useState<UsageActionKind>('correction');
  const [meterCode, setMeterCode] = useState('api_call');
  const [quantityDelta, setQuantityDelta] = useState('-100');
  const [rollupId, setRollupId] = useState('');
  const [confirmCode, setConfirmCode] = useState('');
  const [reason, setReason] = useState('');
  const [result, setResult] = useState<UsageActionResult | null>(null);
  const expectedConfirmCode = expectedUsageConfirmCode(action, meterCode, rollupId);
  const isReasonReady = reason.trim().length >= 12;
  const isConfirmationReady = confirmCode.trim() === expectedConfirmCode;

  const mutation = useMutation({
    mutationFn: () =>
      executeUsageAction(credentials, action, {
        confirmCode,
        meterCode,
        quantityDelta: Number(quantityDelta),
        reason,
        rollupId,
      }),
    onSuccess: async (payload) => {
      setResult(payload);
      await queryClient.invalidateQueries({ queryKey: ['usage-center'] });
    },
  });

  return (
    <div className="mt-4 rounded-md border">
      <div className="flex flex-col gap-2 border-b p-3 md:flex-row md:items-center md:justify-between">
        <div>
          <h3 className="text-sm font-medium">Actions usage</h3>
          <p className="text-muted-foreground text-xs">
            Corrections, freeze meter et replay rollup avec idempotence et audit.
          </p>
        </div>
        <div className="flex flex-wrap gap-2">
          <ActionButton action="correction" current={action} label="Correct" onSelect={setAction} />
          <ActionButton action="freeze" current={action} label="Freeze" onSelect={setAction} />
          <ActionButton action="replay" current={action} label="Replay" onSelect={setAction} />
        </div>
      </div>
      <div className="grid gap-3 p-3 md:grid-cols-2 xl:grid-cols-4">
        {action !== 'replay' ? (
          <Field label="Meter code">
            <Input
              disabled={disabled || mutation.isPending}
              onChange={(event) => setMeterCode(event.target.value)}
              value={meterCode}
            />
          </Field>
        ) : null}
        {action === 'correction' ? (
          <Field label="Quantity delta">
            <Input
              disabled={disabled || mutation.isPending}
              onChange={(event) => setQuantityDelta(event.target.value)}
              type="number"
              value={quantityDelta}
            />
          </Field>
        ) : null}
        {action === 'replay' ? (
          <Field label="Rollup ID">
            <Input
              disabled={disabled || mutation.isPending}
              onChange={(event) => setRollupId(event.target.value)}
              value={rollupId}
            />
          </Field>
        ) : null}
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
              placeholder="ticket USG-123 approved"
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
          disabled={disabled || !isReasonReady || !isConfirmationReady || mutation.isPending}
          onClick={() => mutation.mutate()}
          type="button"
        >
          {actionIcon(action)}
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

type UsageActionPayload = {
  confirmCode: string;
  meterCode: string;
  quantityDelta: number;
  reason: string;
  rollupId: string;
};

function executeUsageAction(
  credentials: AdminCredentials,
  action: UsageActionKind,
  payload: UsageActionPayload,
) {
  if (action === 'correction') {
    return correctUsage(credentials, {
      confirm_code: payload.confirmCode,
      usage_event_id: null,
      meter_code: payload.meterCode,
      quantity_delta: payload.quantityDelta,
      reason: payload.reason,
    });
  }
  if (action === 'freeze') {
    return freezeUsageMeter(credentials, {
      confirm_code: payload.confirmCode,
      meter_code: payload.meterCode,
      reason: payload.reason,
    });
  }
  return replayUsageRollup(credentials, payload.rollupId, {
    confirm_code: payload.confirmCode,
    reason: payload.reason,
  });
}

function expectedUsageConfirmCode(action: UsageActionKind, meterCode: string, rollupId: string) {
  const targetId = action === 'replay' ? rollupId : meterCode;
  return strongConfirmationCode(confirmCodes[action], targetId);
}

function ActionButton({
  action,
  current,
  label,
  onSelect,
}: {
  action: UsageActionKind;
  current: UsageActionKind;
  label: string;
  onSelect: (action: UsageActionKind) => void;
}) {
  return (
    <Button
      onClick={() => onSelect(action)}
      size="sm"
      type="button"
      variant={current === action ? 'default' : 'outline'}
    >
      {actionIcon(action)}
      {label}
    </Button>
  );
}

function Field({ children, label }: { children: ReactNode; label: string }) {
  return (
    <Label className="flex flex-col items-start gap-1 text-xs">
      {label}
      {children}
    </Label>
  );
}

function actionIcon(action: UsageActionKind) {
  if (action === 'correction') return <PencilLine className="size-4" />;
  if (action === 'freeze') return <Ban className="size-4" />;
  return <RotateCcw className="size-4" />;
}
