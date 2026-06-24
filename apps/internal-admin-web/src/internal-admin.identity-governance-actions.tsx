import { useMutation, useQueryClient } from '@tanstack/react-query';
import { Building2, RotateCcw, UserRound } from 'lucide-react';
import type { ReactNode } from 'react';
import { useState } from 'react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Textarea } from '@/components/ui/textarea';
import { cancelRecoveryRequest, revokeBreakGlassAccount } from './internal-admin.api';
import { strongConfirmationCode } from './internal-admin.strong-confirmation';
import type {
  AdminCredentials,
  BreakGlassAccount,
  GovernanceActionResult,
  PendingRecoveryRequest,
} from './internal-admin.types';

type GovernanceTarget =
  | { kind: 'break-glass'; principalId: string; tenantId: string }
  | { kind: 'recovery'; requestId: string };

export function IdentityGovernanceActionLists({
  breakGlassAccounts,
  credentials,
  disabled,
  onSelectTenant,
  onSelectUser,
  recoveryRequests,
}: {
  breakGlassAccounts: BreakGlassAccount[];
  credentials: AdminCredentials;
  disabled: boolean;
  onSelectTenant: (tenantId: string) => void;
  onSelectUser: (principalId: string) => void;
  recoveryRequests: PendingRecoveryRequest[];
}) {
  const queryClient = useQueryClient();
  const [activeTarget, setActiveTarget] = useState<GovernanceTarget | null>(null);
  const [confirmCode, setConfirmCode] = useState('');
  const [reason, setReason] = useState('');
  const [result, setResult] = useState<GovernanceActionResult | null>(null);
  const mutation = useMutation({
    mutationFn: (target: GovernanceTarget) =>
      target.kind === 'break-glass'
        ? revokeBreakGlassAccount(credentials, target.tenantId, target.principalId, {
            confirm_code: confirmCode,
            reason,
          })
        : cancelRecoveryRequest(credentials, target.requestId, {
            confirm_code: confirmCode,
            reason,
          }),
    onSuccess: async (data) => {
      setResult(data);
      setActiveTarget(null);
      setConfirmCode('');
      setReason('');
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ['identity-governance-center'] }),
        queryClient.invalidateQueries({ queryKey: ['internal-audit-events'] }),
        queryClient.invalidateQueries({ queryKey: ['user-detail', data.principal_id] }),
      ]);
    },
  });

  return (
    <>
      <BreakGlassActionList
        activeTarget={activeTarget}
        disabled={disabled}
        error={mutation.error}
        isPending={mutation.isPending}
        onCancel={() => {
          setActiveTarget(null);
          setConfirmCode('');
          setReason('');
        }}
        onConfirmCodeChange={setConfirmCode}
        onReasonChange={setReason}
        onRun={(target) => mutation.mutate(target)}
        onSelectTenant={onSelectTenant}
        onSelectUser={onSelectUser}
        onStart={(target) => {
          setResult(null);
          setActiveTarget(target);
        }}
        confirmCode={confirmCode}
        reason={reason}
        rows={breakGlassAccounts}
      />
      <RecoveryActionList
        activeTarget={activeTarget}
        disabled={disabled}
        error={mutation.error}
        isPending={mutation.isPending}
        onCancel={() => {
          setActiveTarget(null);
          setConfirmCode('');
          setReason('');
        }}
        onConfirmCodeChange={setConfirmCode}
        onReasonChange={setReason}
        onRun={(target) => mutation.mutate(target)}
        onSelectTenant={onSelectTenant}
        onSelectUser={onSelectUser}
        onStart={(target) => {
          setResult(null);
          setActiveTarget(target);
        }}
        confirmCode={confirmCode}
        reason={reason}
        rows={recoveryRequests}
      />
      {result ? (
        <div className="border-primary/20 bg-primary/5 text-primary rounded-md border p-3 text-xs xl:col-span-2">
          {result.audit_action}: {result.object_id}
        </div>
      ) : null}
    </>
  );
}

function BreakGlassActionList(props: SharedActionProps & { rows: BreakGlassAccount[] }) {
  return (
    <GovernanceActionList emptyLabel="Aucun compte break-glass actif." title="Break-glass accounts">
      {props.rows.map((row) => (
        <ActionRow
          {...props}
          badge={row.last_used_at ? `used ${formatDate(row.last_used_at)}` : 'never used'}
          key={`${row.tenant_id}:${row.principal_id}`}
          onSelectTenant={() => props.onSelectTenant(row.tenant_id)}
          onSelectUser={() => props.onSelectUser(row.principal_id)}
          subtitle={`${row.tenant_name} - ${row.procedure_reference}`}
          target={{ kind: 'break-glass', principalId: row.principal_id, tenantId: row.tenant_id }}
          title={row.reason}
          tone="danger"
        />
      ))}
    </GovernanceActionList>
  );
}

function RecoveryActionList(props: SharedActionProps & { rows: PendingRecoveryRequest[] }) {
  return (
    <GovernanceActionList
      emptyLabel="Aucune recovery request pending."
      title="Password recovery queue"
    >
      {props.rows.map((row) => (
        <ActionRow
          {...props}
          badge={row.status}
          key={row.id}
          onSelectTenant={() => props.onSelectTenant(row.tenant_id)}
          onSelectUser={() => props.onSelectUser(row.principal_id)}
          subtitle={`${row.tenant_name} - available ${formatDate(row.available_at)}`}
          target={{ kind: 'recovery', requestId: row.id }}
          title={row.email}
          tone="warning"
        />
      ))}
    </GovernanceActionList>
  );
}

type SharedActionProps = {
  activeTarget: GovernanceTarget | null;
  confirmCode: string;
  disabled: boolean;
  error: Error | null;
  isPending: boolean;
  onCancel: () => void;
  onConfirmCodeChange: (confirmCode: string) => void;
  onReasonChange: (reason: string) => void;
  onRun: (target: GovernanceTarget) => void;
  onSelectTenant: (tenantId: string) => void;
  onSelectUser: (principalId: string) => void;
  onStart: (target: GovernanceTarget) => void;
  reason: string;
};

function ActionRow({
  activeTarget,
  badge,
  confirmCode,
  disabled,
  error,
  isPending,
  onCancel,
  onConfirmCodeChange,
  onReasonChange,
  onRun,
  onSelectTenant,
  onSelectUser,
  onStart,
  reason,
  subtitle,
  target,
  title,
  tone,
}: SharedActionProps & {
  badge: string;
  onSelectTenant: () => void;
  onSelectUser: () => void;
  subtitle: string;
  target: GovernanceTarget;
  title: string;
  tone: 'danger' | 'warning';
}) {
  const isActive = isSameTarget(activeTarget, target);
  const isReasonReady = reason.trim().length >= 12;
  const expectedCode =
    target.kind === 'break-glass'
      ? strongConfirmationCode('REVOKE BREAK GLASS', target.principalId)
      : strongConfirmationCode('CANCEL RECOVERY', target.requestId);
  const isConfirmationReady = confirmCode.trim() === expectedCode;

  return (
    <article className="p-3">
      <div className="mb-2 flex items-start justify-between gap-2">
        <div className="min-w-0">
          <p className="truncate text-sm font-medium">{title}</p>
          <p className="text-muted-foreground truncate text-xs">{subtitle}</p>
        </div>
        <Badge variant={tone === 'danger' ? 'destructive' : 'outline'}>{badge}</Badge>
      </div>
      <div className="flex flex-wrap gap-2">
        <Button
          onClick={() => {
            onSelectTenant();
            window.location.hash = 'tenant-detail';
          }}
          size="sm"
          type="button"
          variant="ghost"
        >
          <Building2 className="size-4" />
          Tenant
        </Button>
        <Button
          onClick={() => {
            onSelectUser();
            window.location.hash = 'user-detail';
          }}
          size="sm"
          type="button"
          variant="outline"
        >
          <UserRound className="size-4" />
          User
        </Button>
        <Button
          disabled={disabled || isPending}
          onClick={() => onStart(target)}
          size="sm"
          type="button"
          variant="destructive"
        >
          {target.kind === 'break-glass' ? 'Revoquer' : 'Cancel'}
        </Button>
      </div>
      {isActive ? (
        <div className="bg-muted/30 mt-3 grid gap-3 rounded-md border p-3">
          <Textarea
            disabled={disabled || isPending}
            onChange={(event) => onReasonChange(event.target.value)}
            placeholder="Motif audit, ticket, incident, approbation..."
            value={reason}
          />
          <Input
            className="font-mono text-xs"
            disabled={disabled || isPending}
            onChange={(event) => onConfirmCodeChange(event.target.value)}
            placeholder={expectedCode}
            value={confirmCode}
          />
          <div className="flex flex-wrap items-center justify-between gap-2">
            <p className="text-muted-foreground text-xs">
              Motif detaille et code exact {expectedCode} requis.
            </p>
            <div className="flex gap-2">
              <Button
                disabled={isPending}
                onClick={onCancel}
                size="sm"
                type="button"
                variant="outline"
              >
                <RotateCcw className="size-4" />
                Annuler
              </Button>
              <Button
                disabled={disabled || !isReasonReady || !isConfirmationReady || isPending}
                onClick={() => onRun(target)}
                size="sm"
                type="button"
                variant="destructive"
              >
                Confirmer
              </Button>
            </div>
          </div>
          {error ? (
            <p className="text-destructive text-xs">
              {error instanceof Error ? error.message : 'Action impossible'}
            </p>
          ) : null}
        </div>
      ) : null}
    </article>
  );
}

function GovernanceActionList({
  children,
  emptyLabel,
  title,
}: {
  children: ReactNode[];
  emptyLabel: string;
  title: string;
}) {
  return (
    <div className="rounded-md border">
      <div className="border-b p-3">
        <h3 className="text-sm font-medium">{title}</h3>
      </div>
      <div className="divide-y">
        {children.length === 0 ? (
          <div className="text-muted-foreground p-4 text-sm">{emptyLabel}</div>
        ) : (
          children
        )}
      </div>
    </div>
  );
}

function isSameTarget(left: GovernanceTarget | null, right: GovernanceTarget): boolean {
  if (!left || left.kind !== right.kind) return false;
  if (left.kind === 'break-glass' && right.kind === 'break-glass') {
    return left.tenantId === right.tenantId && left.principalId === right.principalId;
  }
  return (
    left.kind === 'recovery' && right.kind === 'recovery' && left.requestId === right.requestId
  );
}

function formatDate(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return date.toLocaleString('fr-FR', {
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    month: '2-digit',
  });
}
