import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import {
  Clock3,
  Copy,
  KeyRound,
  RotateCcw,
  ShieldAlert,
  ShieldCheck,
  UserRound,
  Users,
} from 'lucide-react';
import type { ComponentType } from 'react';
import { useState } from 'react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Textarea } from '@/components/ui/textarea';
import { getUserDetail, reactivateUser, suspendUser } from './internal-admin.api';
import { LockedState } from './internal-admin.locked-state';
import type { AdminCredentials, UserLifecycleResult } from './internal-admin.types';

type UserAction = 'reactivate' | 'suspend';

export function UserDetailPanel({
  credentials,
  disabled,
  onSelectTenant,
  onSelectWorkspace,
  principalId,
}: {
  credentials: AdminCredentials;
  disabled: boolean;
  onSelectTenant: (tenantId: string) => void;
  onSelectWorkspace: (workspaceId: string) => void;
  principalId: string | null;
}) {
  const queryClient = useQueryClient();
  const [activeAction, setActiveAction] = useState<UserAction | null>(null);
  const [confirmCode, setConfirmCode] = useState('');
  const [reason, setReason] = useState('');
  const [actionResult, setActionResult] = useState<UserLifecycleResult | null>(null);
  const user = useQuery({
    queryKey: ['user-detail', principalId],
    queryFn: () => getUserDetail(credentials, principalId ?? ''),
    enabled: !disabled && principalId !== null,
  });
  const data = user.data;
  const mutation = useMutation({
    mutationFn: async (action: UserAction) => {
      if (!principalId) throw new Error('User absent.');
      const body = { confirm_code: confirmCode, reason };
      return action === 'suspend'
        ? suspendUser(credentials, principalId, body)
        : reactivateUser(credentials, principalId, body);
    },
    onSuccess: async (result) => {
      setActionResult(result);
      setActiveAction(null);
      setConfirmCode('');
      setReason('');
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ['user-detail', principalId] }),
        queryClient.invalidateQueries({ queryKey: ['security-center'] }),
        queryClient.invalidateQueries({ queryKey: ['access-center'] }),
        queryClient.invalidateQueries({ queryKey: ['global-search'] }),
      ]);
    },
  });
  const availableAction =
    data?.principal_status === 'active' || data?.user_status === 'active'
      ? 'suspend'
      : data?.principal_status === 'suspended' || data?.user_status === 'suspended'
        ? 'reactivate'
        : null;

  return (
    <section className="border-border bg-card mb-5 rounded-lg border p-4" id="user-detail">
      <div className="mb-4 flex flex-col gap-2 md:flex-row md:items-start md:justify-between">
        <div>
          <p className="text-muted-foreground text-xs font-medium tracking-[0.18em] uppercase">
            User detail
          </p>
          <h2 className="text-base font-semibold">
            {data ? data.name : principalId ? 'Chargement user' : 'Aucun user selectionne'}
          </h2>
        </div>
        {data ? (
          <div className="flex flex-wrap gap-2">
            <Badge variant="secondary">{data.user_status}</Badge>
            <Badge variant="outline">{data.principal_status}</Badge>
          </div>
        ) : null}
      </div>
      {disabled ? (
        <LockedState label="Connecte un contexte operateur pour ouvrir une fiche user." />
      ) : null}
      {!disabled && !principalId ? (
        <div className="bg-muted/30 rounded-md border p-4 text-sm">
          Selectionne un user dans Global Search pour ouvrir sa fiche.
        </div>
      ) : null}
      {user.error ? (
        <div className="border-destructive/20 bg-destructive/5 text-destructive rounded-md border p-3 text-sm">
          {user.error instanceof Error ? user.error.message : 'User unavailable'}
        </div>
      ) : null}
      {data ? (
        <div className="space-y-4">
          <div className="grid gap-3 lg:grid-cols-[1.2fr_1fr_1fr]">
            <div className="min-w-0 rounded-md border p-3">
              <p className="text-sm font-medium">{data.email}</p>
              <p className="text-muted-foreground mt-1 truncate font-mono text-xs">
                {data.principal_id}
              </p>
              <Button
                className="mt-3"
                onClick={() => void navigator.clipboard.writeText(data.principal_id)}
                size="sm"
                type="button"
                variant="outline"
              >
                <Copy className="size-4" />
                Copy principal
              </Button>
            </div>
            <LinkCard
              id={data.tenant_id}
              label="Tenant"
              name={data.tenant_name}
              onOpen={() => {
                onSelectTenant(data.tenant_id);
                window.location.hash = 'tenant-detail';
              }}
            />
            <LinkCard
              id={data.primary_workspace_id}
              label="Primary workspace"
              name={data.primary_workspace_name}
              onOpen={() => {
                if (data.primary_workspace_id) {
                  onSelectWorkspace(data.primary_workspace_id);
                  window.location.hash = 'workspace-detail';
                }
              }}
            />
          </div>
          <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-6">
            <UserMetric icon={Users} label="Workspaces" value={formatCount(data.workspace_count)} />
            <UserMetric
              icon={UserRound}
              label="Active access"
              value={formatCount(data.active_workspace_count)}
            />
            <UserMetric
              icon={ShieldCheck}
              label="MFA active"
              tone={data.active_mfa_factor_count === 0 ? 'danger' : 'default'}
              value={formatCount(data.active_mfa_factor_count)}
            />
            <UserMetric
              icon={KeyRound}
              label="OAuth consents"
              value={formatCount(data.active_oauth_consent_count)}
            />
            <UserMetric
              icon={ShieldAlert}
              label="Risk 24h"
              tone={data.risk_events_24h > 0 ? 'danger' : 'default'}
              value={formatCount(data.risk_events_24h)}
            />
            <UserMetric
              icon={Clock3}
              label="Audit 24h"
              value={formatCount(data.audit_events_24h)}
            />
          </div>
          <UserLifecycleActions
            activeAction={activeAction}
            actionResult={actionResult}
            availableAction={availableAction}
            disabled={disabled}
            error={mutation.error}
            isPending={mutation.isPending}
            onCancel={() => {
              setActiveAction(null);
              setConfirmCode('');
              setReason('');
            }}
            onConfirmCodeChange={setConfirmCode}
            onReasonChange={setReason}
            onRun={(action) => mutation.mutate(action)}
            onStart={(action) => {
              setActionResult(null);
              setActiveAction(action);
            }}
            confirmCode={confirmCode}
            principalStatus={data.principal_status}
            reason={reason}
            userStatus={data.user_status}
          />
        </div>
      ) : null}
    </section>
  );
}

function UserLifecycleActions({
  activeAction,
  actionResult,
  availableAction,
  disabled,
  error,
  isPending,
  onCancel,
  onConfirmCodeChange,
  onReasonChange,
  onRun,
  onStart,
  principalStatus,
  reason,
  userStatus,
  confirmCode,
}: {
  activeAction: UserAction | null;
  actionResult: UserLifecycleResult | null;
  availableAction: UserAction | null;
  confirmCode: string;
  disabled: boolean;
  error: Error | null;
  isPending: boolean;
  onCancel: () => void;
  onConfirmCodeChange: (confirmCode: string) => void;
  onReasonChange: (reason: string) => void;
  onRun: (action: UserAction) => void;
  onStart: (action: UserAction) => void;
  principalStatus: string;
  reason: string;
  userStatus: string;
}) {
  const isReasonReady = reason.trim().length >= 12;
  const actionLabel = availableAction === 'suspend' ? 'Suspendre' : 'Reactiver';
  const expectedCode = activeAction === 'suspend' ? 'SUSPEND USER' : 'REACTIVATE USER';
  const isConfirmationReady = confirmCode.trim() === expectedCode;

  return (
    <div className="rounded-md border p-3">
      <div className="mb-3 flex flex-col gap-3 md:flex-row md:items-center md:justify-between">
        <div className="min-w-0">
          <div className="flex items-center gap-2 text-sm font-medium">
            <ShieldAlert className="text-muted-foreground size-4" />
            Cycle de vie utilisateur
          </div>
          <p className="text-muted-foreground mt-1 text-xs">
            Principal: <span className="font-medium">{principalStatus}</span> · User:{' '}
            <span className="font-medium">{userStatus}</span>
          </p>
        </div>
        {availableAction ? (
          <Button
            disabled={disabled || isPending}
            onClick={() => onStart(availableAction)}
            type="button"
            variant={availableAction === 'suspend' ? 'destructive' : 'outline'}
          >
            {availableAction === 'suspend' ? (
              <ShieldAlert className="size-4" />
            ) : (
              <RotateCcw className="size-4" />
            )}
            {actionLabel}
          </Button>
        ) : (
          <Badge variant="outline">Aucune action</Badge>
        )}
      </div>

      {activeAction ? (
        <div className="bg-muted/30 grid gap-3 rounded-md border p-3">
          <Textarea
            disabled={disabled || isPending}
            onChange={(event) => onReasonChange(event.target.value)}
            placeholder="Motif audit, incident, ticket, approbation..."
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
              <Button disabled={isPending} onClick={onCancel} type="button" variant="outline">
                Annuler
              </Button>
              <Button
                disabled={disabled || !isReasonReady || !isConfirmationReady || isPending}
                onClick={() => onRun(activeAction)}
                type="button"
                variant={activeAction === 'suspend' ? 'destructive' : 'default'}
              >
                Confirmer
              </Button>
            </div>
          </div>
        </div>
      ) : null}

      {error ? (
        <p className="text-destructive mt-3 text-xs">
          {error instanceof Error ? error.message : 'Action impossible'}
        </p>
      ) : null}
      {actionResult ? (
        <p className="text-muted-foreground mt-3 text-xs">
          {actionResult.audit_action}: {actionResult.previous_user_status} vers{' '}
          {actionResult.next_user_status}
        </p>
      ) : null}
    </div>
  );
}

function LinkCard({
  id,
  label,
  name,
  onOpen,
}: {
  id: string | null;
  label: string;
  name: string | null;
  onOpen: () => void;
}) {
  return (
    <div className="min-w-0 rounded-md border p-3">
      <p className="text-muted-foreground text-xs">{label}</p>
      <button
        className="hover:text-primary mt-1 text-left text-sm font-medium disabled:pointer-events-none disabled:opacity-60"
        disabled={!id}
        onClick={onOpen}
        type="button"
      >
        {name ?? 'Aucun lien'}
      </button>
      <p className="text-muted-foreground mt-1 truncate font-mono text-xs">{id ?? '-'}</p>
    </div>
  );
}

function UserMetric({
  icon: Icon,
  label,
  tone = 'default',
  value,
}: {
  icon: ComponentType<{ className?: string }>;
  label: string;
  tone?: 'danger' | 'default';
  value: string;
}) {
  return (
    <div className="bg-muted/30 rounded-md border p-3">
      <div className="text-muted-foreground mb-3 flex items-center justify-between text-xs">
        {label}
        <Icon className={tone === 'danger' ? 'text-destructive size-4' : 'size-4'} />
      </div>
      <div
        className={
          tone === 'danger' ? 'text-destructive text-lg font-semibold' : 'text-lg font-semibold'
        }
      >
        {value}
      </div>
    </div>
  );
}

function formatCount(value: number): string {
  return new Intl.NumberFormat('fr-FR').format(value);
}
