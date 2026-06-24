import { useMutation, useQueryClient } from '@tanstack/react-query';
import { MailPlus, MailX, RadioTower, ShieldCheck } from 'lucide-react';
import type { ReactNode } from 'react';
import { useState } from 'react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import {
  replayEmailMessage,
  replayEmailWebhook,
  suppressEmail,
  unsuppressEmail,
} from './internal-admin.api';
import { strongConfirmationCode } from './internal-admin.strong-confirmation';
import type { AdminCredentials, CommunicationsActionResult } from './internal-admin.types';

type CommunicationsActionKind = 'replay-email' | 'replay-webhook' | 'suppress' | 'unsuppress';

const confirmCodes: Record<CommunicationsActionKind, string> = {
  'replay-email': 'REPLAY EMAIL',
  'replay-webhook': 'REPLAY WEBHOOK',
  suppress: 'SUPPRESS EMAIL',
  unsuppress: 'UNSUPPRESS EMAIL',
};

export function CommunicationsActionsPanel({
  credentials,
  disabled,
}: {
  credentials: AdminCredentials;
  disabled: boolean;
}) {
  const queryClient = useQueryClient();
  const [action, setAction] = useState<CommunicationsActionKind>('replay-email');
  const [messageId, setMessageId] = useState('');
  const [eventId, setEventId] = useState('');
  const [email, setEmail] = useState('');
  const [confirmCode, setConfirmCode] = useState('');
  const [reason, setReason] = useState('');
  const [result, setResult] = useState<CommunicationsActionResult | null>(null);
  const expectedConfirmCode = expectedCommunicationsConfirmCode(
    credentials,
    action,
    messageId,
    eventId,
  );

  const mutation = useMutation({
    mutationFn: () =>
      executeCommunicationsAction(credentials, action, {
        confirmCode,
        email,
        eventId,
        messageId,
        reason,
      }),
    onSuccess: async (payload) => {
      setResult(payload);
      await queryClient.invalidateQueries({ queryKey: ['communications-center'] });
    },
  });

  return (
    <div className="mt-4 rounded-md border">
      <div className="flex flex-col gap-2 border-b p-3 md:flex-row md:items-center md:justify-between">
        <div>
          <h3 className="text-sm font-medium">Actions communications</h3>
          <p className="text-muted-foreground text-xs">
            Replay email/webhook et suppression d'adresse avec audit.
          </p>
        </div>
        <div className="flex flex-wrap gap-2">
          <ActionButton action="replay-email" current={action} label="Email" onSelect={setAction} />
          <ActionButton
            action="replay-webhook"
            current={action}
            label="Webhook"
            onSelect={setAction}
          />
          <ActionButton action="suppress" current={action} label="Suppress" onSelect={setAction} />
          <ActionButton
            action="unsuppress"
            current={action}
            label="Unsuppress"
            onSelect={setAction}
          />
        </div>
      </div>
      <div className="grid gap-3 p-3 md:grid-cols-2 xl:grid-cols-4">
        {action === 'replay-email' ? (
          <Field label="Email message ID">
            <Input
              disabled={disabled || mutation.isPending}
              onChange={(event) => setMessageId(event.target.value)}
              value={messageId}
            />
          </Field>
        ) : null}
        {action === 'replay-webhook' ? (
          <Field label="Webhook event ID">
            <Input
              disabled={disabled || mutation.isPending}
              onChange={(event) => setEventId(event.target.value)}
              value={eventId}
            />
          </Field>
        ) : null}
        {action === 'suppress' || action === 'unsuppress' ? (
          <Field label="Email">
            <Input
              disabled={disabled || mutation.isPending}
              onChange={(event) => setEmail(event.target.value)}
              value={email}
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
              placeholder="ticket COMMS-123 approved"
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

type CommunicationsActionPayload = {
  confirmCode: string;
  email: string;
  eventId: string;
  messageId: string;
  reason: string;
};

function executeCommunicationsAction(
  credentials: AdminCredentials,
  action: CommunicationsActionKind,
  payload: CommunicationsActionPayload,
) {
  const body = { confirm_code: payload.confirmCode, reason: payload.reason };
  if (action === 'replay-email') return replayEmailMessage(credentials, payload.messageId, body);
  if (action === 'replay-webhook') return replayEmailWebhook(credentials, payload.eventId, body);
  const emailBody = { ...body, email: payload.email };
  if (action === 'suppress') return suppressEmail(credentials, emailBody);
  return unsuppressEmail(credentials, emailBody);
}

function expectedCommunicationsConfirmCode(
  credentials: AdminCredentials,
  action: CommunicationsActionKind,
  messageId: string,
  eventId: string,
) {
  if (action === 'replay-email') return strongConfirmationCode(confirmCodes[action], messageId);
  if (action === 'replay-webhook') return strongConfirmationCode(confirmCodes[action], eventId);
  return strongConfirmationCode(confirmCodes[action], credentials.workspaceId);
}

function ActionButton({
  action,
  current,
  label,
  onSelect,
}: {
  action: CommunicationsActionKind;
  current: CommunicationsActionKind;
  label: string;
  onSelect: (action: CommunicationsActionKind) => void;
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

function actionIcon(action: CommunicationsActionKind) {
  if (action === 'replay-email') return <MailPlus className="size-4" />;
  if (action === 'replay-webhook') return <RadioTower className="size-4" />;
  if (action === 'suppress') return <MailX className="size-4" />;
  return <ShieldCheck className="size-4" />;
}
