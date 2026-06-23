import { useMutation, useQueryClient } from '@tanstack/react-query';
import { BadgeCheck, PackageMinus, PackagePlus, UploadCloud } from 'lucide-react';
import type { ReactNode } from 'react';
import { useState } from 'react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import {
  grantEntitlementFeature,
  overrideEntitlementQuota,
  publishEntitlementChanges,
  revokeEntitlementFeature,
} from './internal-admin.api';
import type { AdminCredentials, EntitlementActionResult } from './internal-admin.types';

type ActionKind = 'grant' | 'publish' | 'quota' | 'revoke';

const confirmCodes: Record<ActionKind, string> = {
  grant: 'GRANT FEATURE',
  revoke: 'REVOKE FEATURE',
  quota: 'OVERRIDE QUOTA',
  publish: 'PUBLISH ENTITLEMENTS',
};

export function EntitlementsActionsPanel({
  credentials,
  disabled,
}: {
  credentials: AdminCredentials;
  disabled: boolean;
}) {
  const queryClient = useQueryClient();
  const [action, setAction] = useState<ActionKind>('grant');
  const [featureCode, setFeatureCode] = useState('advanced_search');
  const [quotaCode, setQuotaCode] = useState('seats');
  const [includedQuantity, setIncludedQuantity] = useState('100');
  const [confirmCode, setConfirmCode] = useState('');
  const [reason, setReason] = useState('');
  const [result, setResult] = useState<EntitlementActionResult | null>(null);

  const mutation = useMutation({
    mutationFn: () => executeAction(credentials, action, formPayload()),
    onSuccess: async (payload) => {
      setResult(payload);
      await queryClient.invalidateQueries({ queryKey: ['entitlements-center'] });
    },
  });

  function formPayload(): EntitlementFormPayload {
    return {
      confirmCode,
      featureCode,
      includedQuantity: Number(includedQuantity),
      quotaCode,
      reason,
    };
  }

  return (
    <div className="mt-4 rounded-md border">
      <div className="flex flex-col gap-2 border-b p-3 md:flex-row md:items-center md:justify-between">
        <div>
          <h3 className="text-sm font-medium">Actions entitlements</h3>
          <p className="text-muted-foreground text-xs">
            Grant, revoke, override quota et publication avec confirmation forte.
          </p>
        </div>
        <div className="flex flex-wrap gap-2">
          <ActionButton action="grant" current={action} label="Grant" onSelect={setAction} />
          <ActionButton action="revoke" current={action} label="Revoke" onSelect={setAction} />
          <ActionButton action="quota" current={action} label="Quota" onSelect={setAction} />
          <ActionButton action="publish" current={action} label="Publish" onSelect={setAction} />
        </div>
      </div>
      <div className="grid gap-3 p-3 md:grid-cols-2 xl:grid-cols-4">
        {action === 'grant' || action === 'revoke' ? (
          <Field label="Feature code">
            <Input
              disabled={disabled || mutation.isPending}
              onChange={(event) => setFeatureCode(event.target.value)}
              value={featureCode}
            />
          </Field>
        ) : null}
        {action === 'quota' ? (
          <>
            <Field label="Quota code">
              <Input
                disabled={disabled || mutation.isPending}
                onChange={(event) => setQuotaCode(event.target.value)}
                value={quotaCode}
              />
            </Field>
            <Field label="Included quantity">
              <Input
                disabled={disabled || mutation.isPending}
                min={0}
                onChange={(event) => setIncludedQuantity(event.target.value)}
                type="number"
                value={includedQuantity}
              />
            </Field>
          </>
        ) : null}
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
              placeholder="ticket ENT-123 approved"
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

type EntitlementFormPayload = {
  confirmCode: string;
  featureCode: string;
  includedQuantity: number;
  quotaCode: string;
  reason: string;
};

function executeAction(
  credentials: AdminCredentials,
  action: ActionKind,
  payload: EntitlementFormPayload,
) {
  if (action === 'grant') {
    return grantEntitlementFeature(credentials, {
      confirm_code: payload.confirmCode,
      feature_code: payload.featureCode,
      value: { enabled: true },
      reason: payload.reason,
    });
  }
  if (action === 'revoke') {
    return revokeEntitlementFeature(credentials, {
      confirm_code: payload.confirmCode,
      feature_code: payload.featureCode,
      value: { enabled: false },
      reason: payload.reason,
    });
  }
  if (action === 'quota') {
    return overrideEntitlementQuota(credentials, {
      confirm_code: payload.confirmCode,
      quota_code: payload.quotaCode,
      included_quantity: payload.includedQuantity,
      reason: payload.reason,
    });
  }
  return publishEntitlementChanges(credentials, {
    confirm_code: payload.confirmCode,
    reason: payload.reason,
  });
}

function ActionButton({
  action,
  current,
  label,
  onSelect,
}: {
  action: ActionKind;
  current: ActionKind;
  label: string;
  onSelect: (action: ActionKind) => void;
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

function actionIcon(action: ActionKind) {
  if (action === 'grant') return <PackagePlus className="size-4" />;
  if (action === 'revoke') return <PackageMinus className="size-4" />;
  if (action === 'quota') return <BadgeCheck className="size-4" />;
  return <UploadCloud className="size-4" />;
}
