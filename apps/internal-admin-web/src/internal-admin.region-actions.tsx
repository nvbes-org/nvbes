import { useMutation, useQueryClient } from '@tanstack/react-query';
import { AlertTriangle, MapPinned } from 'lucide-react';
import type { ReactNode } from 'react';
import { useState } from 'react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import { flagRegionResidency, recordRegionException } from './internal-admin.api';
import { strongConfirmationCode } from './internal-admin.strong-confirmation';
import type { AdminCredentials, RegionActionResult } from './internal-admin.types';

type RegionActionKind = 'exception' | 'flag';

const confirmCodes: Record<RegionActionKind, string> = {
  flag: 'FLAG RESIDENCY',
  exception: 'RECORD REGION EXCEPTION',
};

export function RegionActionsPanel({
  credentials,
  disabled,
}: {
  credentials: AdminCredentials;
  disabled: boolean;
}) {
  const queryClient = useQueryClient();
  const [action, setAction] = useState<RegionActionKind>('flag');
  const [targetWorkspaceId, setTargetWorkspaceId] = useState(credentials.workspaceId);
  const [dataRegion, setDataRegion] = useState('eu');
  const [jurisdiction, setJurisdiction] = useState('gdpr');
  const [exceptionKind, setExceptionKind] = useState('data_residency_exception');
  const [confirmCode, setConfirmCode] = useState('');
  const [reason, setReason] = useState('');
  const [result, setResult] = useState<RegionActionResult | null>(null);
  const requiredConfirmCode = regionConfirmCode(action, targetWorkspaceId);

  const mutation = useMutation({
    mutationFn: () =>
      executeRegionAction(credentials, action, {
        confirmCode,
        dataRegion,
        exceptionKind,
        jurisdiction,
        reason,
        targetWorkspaceId,
      }),
    onSuccess: async (payload) => {
      setResult(payload);
      await queryClient.invalidateQueries({ queryKey: ['region-center'] });
    },
  });

  return (
    <div className="mt-4 rounded-md border">
      <div className="flex flex-col gap-2 border-b p-3 md:flex-row md:items-center md:justify-between">
        <div>
          <h3 className="text-sm font-medium">Actions region</h3>
          <p className="text-muted-foreground text-xs">
            Flag de residence et exception data residency avec audit.
          </p>
        </div>
        <div className="flex flex-wrap gap-2">
          <ActionButton action="flag" current={action} label="Flag" onSelect={setAction} />
          <ActionButton
            action="exception"
            current={action}
            label="Exception"
            onSelect={setAction}
          />
        </div>
      </div>
      <div className="grid gap-3 p-3 md:grid-cols-2 xl:grid-cols-4">
        <Field label="Target workspace ID">
          <Input
            disabled={disabled || mutation.isPending}
            onChange={(event) => setTargetWorkspaceId(event.target.value)}
            value={targetWorkspaceId}
          />
        </Field>
        {action === 'flag' ? (
          <>
            <Field label="Data region">
              <Input
                disabled={disabled || mutation.isPending}
                onChange={(event) => setDataRegion(event.target.value)}
                value={dataRegion}
              />
            </Field>
            <Field label="Jurisdiction">
              <Input
                disabled={disabled || mutation.isPending}
                onChange={(event) => setJurisdiction(event.target.value)}
                value={jurisdiction}
              />
            </Field>
          </>
        ) : (
          <Field label="Exception kind">
            <Input
              disabled={disabled || mutation.isPending}
              onChange={(event) => setExceptionKind(event.target.value)}
              value={exceptionKind}
            />
          </Field>
        )}
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
              placeholder="ticket REG-123 approved"
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
          disabled={disabled || mutation.isPending}
          onClick={() => mutation.mutate()}
          type="button"
        >
          {action === 'flag' ? (
            <MapPinned className="size-4" />
          ) : (
            <AlertTriangle className="size-4" />
          )}
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

function regionConfirmCode(action: RegionActionKind, targetWorkspaceId: string): string {
  const baseCode = confirmCodes[action];
  if (action !== 'exception') return baseCode;
  return strongConfirmationCode(baseCode, targetWorkspaceId);
}

type RegionActionPayload = {
  confirmCode: string;
  dataRegion: string;
  exceptionKind: string;
  jurisdiction: string;
  reason: string;
  targetWorkspaceId: string;
};

function executeRegionAction(
  credentials: AdminCredentials,
  action: RegionActionKind,
  payload: RegionActionPayload,
) {
  if (action === 'flag') {
    return flagRegionResidency(credentials, payload.targetWorkspaceId, {
      confirm_code: payload.confirmCode,
      data_region: payload.dataRegion,
      jurisdiction: payload.jurisdiction,
      reason: payload.reason,
    });
  }
  return recordRegionException(credentials, payload.targetWorkspaceId, {
    confirm_code: payload.confirmCode,
    exception_kind: payload.exceptionKind,
    reason: payload.reason,
  });
}

function ActionButton({
  action,
  current,
  label,
  onSelect,
}: {
  action: RegionActionKind;
  current: RegionActionKind;
  label: string;
  onSelect: (action: RegionActionKind) => void;
}) {
  return (
    <Button
      onClick={() => onSelect(action)}
      size="sm"
      type="button"
      variant={current === action ? 'default' : 'outline'}
    >
      {action === 'flag' ? <MapPinned className="size-4" /> : <AlertTriangle className="size-4" />}
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
