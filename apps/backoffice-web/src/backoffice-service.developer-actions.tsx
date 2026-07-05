import { useMutation, useQueryClient } from '@tanstack/react-query';
import { KeyRound, ShieldOff, Store } from 'lucide-react';
import type { ReactNode } from 'react';
import { useState } from 'react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import {
  approveMarketplaceApp,
  revokeDeveloperClient,
  rotateDeveloperSecret,
} from './backoffice-service.api';
import { strongConfirmationCode } from './backoffice-service.strong-confirmation';
import type { AdminCredentials, DeveloperActionResult } from './backoffice-service.types';

type DeveloperActionKind = 'approve' | 'revoke' | 'rotate';

const confirmCodes: Record<DeveloperActionKind, string> = {
  revoke: 'REVOKE CLIENT',
  rotate: 'ROTATE SECRET',
  approve: 'APPROVE MARKETPLACE APP',
};

export function DeveloperActionsPanel({
  credentials,
  disabled,
}: {
  credentials: AdminCredentials;
  disabled: boolean;
}) {
  const queryClient = useQueryClient();
  const [action, setAction] = useState<DeveloperActionKind>('revoke');
  const [clientId, setClientId] = useState('');
  const [appId, setAppId] = useState('');
  const [confirmCode, setConfirmCode] = useState('');
  const [reason, setReason] = useState('');
  const [result, setResult] = useState<DeveloperActionResult | null>(null);
  const expectedConfirmCode = expectedDeveloperConfirmCode(action, clientId, appId);
  const isReasonReady = reason.trim().length >= 12;
  const isConfirmationReady = confirmCode.trim() === expectedConfirmCode;

  const mutation = useMutation({
    mutationFn: () =>
      executeDeveloperAction(credentials, action, {
        appId,
        clientId,
        confirmCode,
        reason,
      }),
    onSuccess: async (payload) => {
      setResult(payload);
      await queryClient.invalidateQueries({ queryKey: ['developer-center'] });
    },
  });

  return (
    <div className="mt-4 rounded-md border">
      <div className="flex flex-col gap-2 border-b p-3 md:flex-row md:items-center md:justify-between">
        <div>
          <h3 className="text-sm font-medium">Actions developer</h3>
          <p className="text-muted-foreground text-xs">
            Revoke client, rotate secret et approval marketplace avec audit.
          </p>
        </div>
        <div className="flex flex-wrap gap-2">
          <ActionButton action="revoke" current={action} label="Revoke" onSelect={setAction} />
          <ActionButton action="rotate" current={action} label="Rotate" onSelect={setAction} />
          <ActionButton action="approve" current={action} label="Approve" onSelect={setAction} />
        </div>
      </div>
      <div className="grid gap-3 p-3 md:grid-cols-2 xl:grid-cols-4">
        {action === 'approve' ? (
          <Field label="Marketplace app ID">
            <Input
              disabled={disabled || mutation.isPending}
              onChange={(event) => setAppId(event.target.value)}
              value={appId}
            />
          </Field>
        ) : (
          <Field label="Client ID">
            <Input
              disabled={disabled || mutation.isPending}
              onChange={(event) => setClientId(event.target.value)}
              value={clientId}
            />
          </Field>
        )}
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
              placeholder="ticket DEV-123 approved"
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

type DeveloperActionPayload = {
  appId: string;
  clientId: string;
  confirmCode: string;
  reason: string;
};

function executeDeveloperAction(
  credentials: AdminCredentials,
  action: DeveloperActionKind,
  payload: DeveloperActionPayload,
) {
  const body = { confirm_code: payload.confirmCode, reason: payload.reason };
  if (action === 'revoke') return revokeDeveloperClient(credentials, payload.clientId, body);
  if (action === 'rotate') return rotateDeveloperSecret(credentials, payload.clientId, body);
  return approveMarketplaceApp(credentials, payload.appId, body);
}

function expectedDeveloperConfirmCode(
  action: DeveloperActionKind,
  clientId: string,
  appId: string,
) {
  const targetId = action === 'approve' ? appId : clientId;
  return strongConfirmationCode(confirmCodes[action], targetId);
}

function ActionButton({
  action,
  current,
  label,
  onSelect,
}: {
  action: DeveloperActionKind;
  current: DeveloperActionKind;
  label: string;
  onSelect: (action: DeveloperActionKind) => void;
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

function actionIcon(action: DeveloperActionKind) {
  if (action === 'revoke') return <ShieldOff className="size-4" />;
  if (action === 'rotate') return <KeyRound className="size-4" />;
  return <Store className="size-4" />;
}
