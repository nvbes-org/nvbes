import { useMutation, useQueryClient } from '@tanstack/react-query';
import { AlertTriangle, CalendarClock, CheckCircle2, FileDown, RefreshCcw } from 'lucide-react';
import type { ReactNode } from 'react';
import { useState } from 'react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import {
  executeOperationsAction,
  operationsConfirmCodes,
  parseIncidentStatus,
  targetLabel,
  type IncidentStatus,
  type OperationsActionKind,
} from './internal-admin.operations-action-model';
import { strongConfirmationCode } from './internal-admin.strong-confirmation';
import type { AdminCredentials, OperationsActionResult } from './internal-admin.types';

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
  const [incidentStatus, setIncidentStatus] = useState<IncidentStatus>('mitigating');
  const [maintenanceTitle, setMaintenanceTitle] = useState('');
  const [scheduledStart, setScheduledStart] = useState('');
  const [scheduledEnd, setScheduledEnd] = useState('');
  const [confirmCode, setConfirmCode] = useState('');
  const [reason, setReason] = useState('');
  const [result, setResult] = useState<OperationsActionResult | null>(null);
  const confirmationTarget = action === 'maintenanceWindow' ? credentials.workspaceId : targetId;
  const expectedConfirmCode = strongConfirmationCode(
    operationsConfirmCodes[action],
    confirmationTarget,
  );
  const isReasonReady = reason.trim().length >= 12;
  const isConfirmationReady = confirmCode.trim() === expectedConfirmCode;
  const isTargetReady = action === 'maintenanceWindow' ? true : targetId.trim().length > 0;
  const isMaintenanceReady =
    action !== 'maintenanceWindow' ||
    (maintenanceTitle.trim().length >= 4 &&
      scheduledStart.trim().length > 0 &&
      scheduledEnd.trim().length > 0);

  const mutation = useMutation({
    mutationFn: () =>
      executeOperationsAction(credentials, action, {
        confirmCode,
        incidentStatus,
        maintenanceTitle,
        reason,
        scheduledEnd,
        scheduledStart,
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
            action="incidentState"
            current={action}
            label="Incident"
            onSelect={setAction}
          />
          <ActionButton
            action="maintenanceWindow"
            current={action}
            label="Maintenance"
            onSelect={setAction}
          />
          <ActionButton action="replayJob" current={action} label="Job run" onSelect={setAction} />
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
        {action === 'maintenanceWindow' ? (
          <>
            <Field label="Maintenance title">
              <Input
                disabled={disabled || mutation.isPending}
                onChange={(event) => setMaintenanceTitle(event.target.value)}
                value={maintenanceTitle}
              />
            </Field>
            <Field label="Start time">
              <Input
                disabled={disabled || mutation.isPending}
                onChange={(event) => setScheduledStart(event.target.value)}
                placeholder="2026-07-01T02:00:00Z"
                value={scheduledStart}
              />
            </Field>
            <Field label="End time">
              <Input
                disabled={disabled || mutation.isPending}
                onChange={(event) => setScheduledEnd(event.target.value)}
                placeholder="2026-07-01T04:00:00Z"
                value={scheduledEnd}
              />
            </Field>
          </>
        ) : (
          <Field label={targetLabel(action)}>
            <Input
              disabled={disabled || mutation.isPending}
              onChange={(event) => setTargetId(event.target.value)}
              value={targetId}
            />
          </Field>
        )}
        {action === 'incidentState' ? (
          <Field label="Incident status">
            <Input
              disabled={disabled || mutation.isPending}
              onChange={(event) => setIncidentStatus(parseIncidentStatus(event.target.value))}
              placeholder="open, mitigating, resolved"
              value={incidentStatus}
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
          disabled={
            disabled ||
            !isTargetReady ||
            !isMaintenanceReady ||
            !isReasonReady ||
            !isConfirmationReady ||
            mutation.isPending
          }
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
  if (action === 'incidentState') return <AlertTriangle className="size-4" />;
  if (action === 'maintenanceWindow') return <CalendarClock className="size-4" />;
  if (action === 'replayExport') return <FileDown className="size-4" />;
  if (action === 'resolveRecon') return <CheckCircle2 className="size-4" />;
  return <RefreshCcw className="size-4" />;
}

function Field({ children, label }: { children: ReactNode; label: string }) {
  return (
    <Label className="flex flex-col items-start gap-1 text-xs">
      {label}
      {children}
    </Label>
  );
}
