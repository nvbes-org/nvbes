import { useMutation, useQueryClient } from '@tanstack/react-query';
import { FileX2, ShieldCheck, UserX } from 'lucide-react';
import type { ReactNode } from 'react';
import { useState } from 'react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import {
  requestComplianceErasure,
  reviewComplianceSuppression,
  revokeComplianceConsent,
} from './internal-admin.api';
import { strongConfirmationCode } from './internal-admin.strong-confirmation';
import type { AdminCredentials, ComplianceActionResult } from './internal-admin.types';

type ComplianceActionKind = 'erasure' | 'review-suppression' | 'revoke-consent';

const confirmCodes: Record<ComplianceActionKind, string> = {
  'revoke-consent': 'REVOKE CONSENT',
  erasure: 'REQUEST ERASURE',
  'review-suppression': 'REVIEW SUPPRESSION',
};

export function ComplianceActionsPanel({
  credentials,
  disabled,
}: {
  credentials: AdminCredentials;
  disabled: boolean;
}) {
  const queryClient = useQueryClient();
  const [action, setAction] = useState<ComplianceActionKind>('revoke-consent');
  const [consentId, setConsentId] = useState('');
  const [principalId, setPrincipalId] = useState('');
  const [email, setEmail] = useState('');
  const [confirmCode, setConfirmCode] = useState('');
  const [reason, setReason] = useState('');
  const [result, setResult] = useState<ComplianceActionResult | null>(null);
  const requiredConfirmCode = complianceConfirmCode(action, {
    consentId,
    email,
    principalId,
  });
  const isReasonReady = reason.trim().length >= 12;
  const isConfirmationReady = confirmCode.trim() === requiredConfirmCode;

  const mutation = useMutation({
    mutationFn: () =>
      executeComplianceAction(credentials, action, {
        confirmCode,
        consentId,
        email,
        principalId,
        reason,
      }),
    onSuccess: async (payload) => {
      setResult(payload);
      await queryClient.invalidateQueries({ queryKey: ['compliance-center'] });
    },
  });

  return (
    <div className="mt-4 rounded-md border">
      <div className="flex flex-col gap-2 border-b p-3 md:flex-row md:items-center md:justify-between">
        <div>
          <h3 className="text-sm font-medium">Actions compliance</h3>
          <p className="text-muted-foreground text-xs">
            Consent, effacement et revue suppression avec confirmation forte.
          </p>
        </div>
        <div className="flex flex-wrap gap-2">
          <ActionButton
            action="revoke-consent"
            current={action}
            label="Consent"
            onSelect={setAction}
          />
          <ActionButton action="erasure" current={action} label="Erasure" onSelect={setAction} />
          <ActionButton
            action="review-suppression"
            current={action}
            label="Review"
            onSelect={setAction}
          />
        </div>
      </div>
      <div className="grid gap-3 p-3 md:grid-cols-2 xl:grid-cols-4">
        {action === 'revoke-consent' ? (
          <Field label="Consent ID">
            <Input
              disabled={disabled || mutation.isPending}
              onChange={(event) => setConsentId(event.target.value)}
              value={consentId}
            />
          </Field>
        ) : null}
        {action === 'erasure' ? (
          <Field label="Principal ID">
            <Input
              disabled={disabled || mutation.isPending}
              onChange={(event) => setPrincipalId(event.target.value)}
              value={principalId}
            />
          </Field>
        ) : null}
        {action === 'review-suppression' ? (
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
            placeholder={requiredConfirmCode}
            value={confirmCode}
          />
        </Field>
        <div className="md:col-span-2 xl:col-span-4">
          <Field label="Reason">
            <Textarea
              disabled={disabled || mutation.isPending}
              onChange={(event) => setReason(event.target.value)}
              placeholder="ticket GDPR-123 approved"
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

function complianceConfirmCode(
  action: ComplianceActionKind,
  targets: { consentId: string; email: string; principalId: string },
): string {
  const baseCode = confirmCodes[action];
  if (action === 'revoke-consent') return strongConfirmationCode(baseCode, targets.consentId);
  if (action === 'review-suppression') return strongConfirmationCode(baseCode, targets.email);
  return strongConfirmationCode(baseCode, targets.principalId);
}

type ComplianceActionPayload = {
  confirmCode: string;
  consentId: string;
  email: string;
  principalId: string;
  reason: string;
};

function executeComplianceAction(
  credentials: AdminCredentials,
  action: ComplianceActionKind,
  payload: ComplianceActionPayload,
) {
  const body = { confirm_code: payload.confirmCode, reason: payload.reason };
  if (action === 'revoke-consent') {
    return revokeComplianceConsent(credentials, payload.consentId, body);
  }
  if (action === 'erasure') {
    return requestComplianceErasure(credentials, payload.principalId, body);
  }
  return reviewComplianceSuppression(credentials, { ...body, email: payload.email });
}

function ActionButton({
  action,
  current,
  label,
  onSelect,
}: {
  action: ComplianceActionKind;
  current: ComplianceActionKind;
  label: string;
  onSelect: (action: ComplianceActionKind) => void;
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

function actionIcon(action: ComplianceActionKind) {
  if (action === 'revoke-consent') return <FileX2 className="size-4" />;
  if (action === 'erasure') return <UserX className="size-4" />;
  return <ShieldCheck className="size-4" />;
}
