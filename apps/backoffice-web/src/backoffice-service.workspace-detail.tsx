import { ClipboardButton } from '@nvbes/web-ui';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import {
  BadgeCheck,
  Building2,
  Clock3,
  CreditCard,
  RotateCcw,
  ShieldAlert,
  ShieldCheck,
  Users,
} from 'lucide-react';
import type { ComponentType } from 'react';
import { useState } from 'react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Textarea } from '@/components/ui/textarea';
import {
  getWorkspaceDetail,
  reactivateWorkspace,
  suspendWorkspace,
} from './backoffice-service.api';
import { workspaceDetailTabs } from './backoffice-service.detail-tab-builders';
import { EntityDetailTabs } from './backoffice-service.detail-tabs';
import { LockedState } from './backoffice-service.locked-state';
import { strongConfirmationCode } from './backoffice-service.strong-confirmation';
import type { AdminCredentials, WorkspaceLifecycleResult } from './backoffice-service.types';

type WorkspaceAction = 'reactivate' | 'suspend';

export function WorkspaceDetailPanel({
  credentials,
  disabled,
  onSelectTenant,
  workspaceId,
}: {
  credentials: AdminCredentials;
  disabled: boolean;
  onSelectTenant: (tenantId: string) => void;
  workspaceId: string | null;
}) {
  const queryClient = useQueryClient();
  const [activeAction, setActiveAction] = useState<WorkspaceAction | null>(null);
  const [confirmCode, setConfirmCode] = useState('');
  const [reason, setReason] = useState('');
  const [actionResult, setActionResult] = useState<WorkspaceLifecycleResult | null>(null);
  const workspace = useQuery({
    queryKey: ['workspace-detail', workspaceId],
    queryFn: () => getWorkspaceDetail(credentials, workspaceId ?? ''),
    enabled: !disabled && workspaceId !== null,
  });
  const data = workspace.data;
  const mutation = useMutation({
    mutationFn: async (action: WorkspaceAction) => {
      if (!workspaceId) throw new Error('Workspace absent.');
      const body = { confirm_code: confirmCode, reason };
      return action === 'suspend'
        ? suspendWorkspace(credentials, workspaceId, body)
        : reactivateWorkspace(credentials, workspaceId, body);
    },
    onSuccess: async (result) => {
      setActionResult(result);
      setActiveAction(null);
      setConfirmCode('');
      setReason('');
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ['workspace-detail', workspaceId] }),
        queryClient.invalidateQueries({ queryKey: ['customer-center'] }),
        queryClient.invalidateQueries({ queryKey: ['operations-center'] }),
        queryClient.invalidateQueries({ queryKey: ['global-search'] }),
      ]);
    },
  });
  const availableAction =
    data?.status === 'active' ? 'suspend' : data?.status === 'suspended' ? 'reactivate' : null;

  return (
    <section className="border-border bg-card mb-5 rounded-lg border p-4" id="workspace-detail">
      <div className="mb-4 flex flex-col gap-2 md:flex-row md:items-start md:justify-between">
        <div>
          <p className="text-muted-foreground text-xs font-medium tracking-[0.18em] uppercase">
            Workspace detail
          </p>
          <h2 className="text-base font-semibold">
            {data
              ? data.name
              : workspaceId
                ? 'Chargement workspace'
                : 'Aucun workspace selectionne'}
          </h2>
        </div>
        {data ? (
          <div className="flex flex-wrap gap-2">
            <Badge variant="secondary">{data.status}</Badge>
            <Badge variant="outline">{data.workspace_type}</Badge>
            <Badge variant="outline">{data.plan_code}</Badge>
          </div>
        ) : null}
      </div>
      {disabled ? (
        <LockedState label="Connecte un contexte operateur pour ouvrir une fiche workspace." />
      ) : null}
      {!disabled && !workspaceId ? (
        <div className="bg-muted/30 rounded-md border p-4 text-sm">
          Selectionne un workspace dans Global Search pour ouvrir sa fiche.
        </div>
      ) : null}
      {workspace.error ? (
        <div className="border-destructive/20 bg-destructive/5 text-destructive rounded-md border p-3 text-sm">
          {workspace.error instanceof Error ? workspace.error.message : 'Workspace unavailable'}
        </div>
      ) : null}
      {data ? (
        <div className="space-y-4">
          <div className="grid gap-3 md:grid-cols-[1fr_auto]">
            <div className="min-w-0 rounded-md border p-3">
              <p className="text-muted-foreground text-xs">Tenant</p>
              <button
                className="hover:text-primary mt-1 text-left text-sm font-medium"
                onClick={() => {
                  onSelectTenant(data.tenant_id);
                  window.location.hash = 'tenant-detail';
                }}
                type="button"
              >
                {data.tenant_name}
              </button>
              <p className="text-muted-foreground mt-1 truncate font-mono text-xs">
                {data.tenant_id}
              </p>
            </div>
            <div className="flex items-center justify-between gap-3 rounded-md border p-3 md:min-w-80">
              <div className="min-w-0">
                <p className="text-sm font-medium">Workspace ID</p>
                <p className="text-muted-foreground truncate font-mono text-xs">{data.id}</p>
              </div>
              <ClipboardButton label="Copy ID" value={data.id} />
            </div>
          </div>
          <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-7">
            <WorkspaceMetric icon={Users} label="Members" value={formatCount(data.member_count)} />
            <WorkspaceMetric
              icon={BadgeCheck}
              label="Owners"
              value={formatCount(data.owner_count)}
            />
            <WorkspaceMetric
              icon={ShieldCheck}
              label="Active"
              value={formatCount(data.active_member_count)}
            />
            <WorkspaceMetric
              icon={Building2}
              label="Service accounts"
              value={formatCount(data.service_account_count)}
            />
            <WorkspaceMetric
              icon={Clock3}
              label="Audit 24h"
              value={formatCount(data.audit_events_24h)}
            />
            <WorkspaceMetric
              icon={CreditCard}
              label="Active subs"
              value={formatCount(data.active_subscription_count)}
            />
            <WorkspaceMetric
              icon={CreditCard}
              label="Open invoices"
              tone={data.open_invoice_count > 0 ? 'danger' : 'default'}
              value={formatCount(data.open_invoice_count)}
            />
          </div>
          <EntityDetailTabs tabs={workspaceDetailTabs(data)} />
          <WorkspaceLifecycleActions
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
            reason={reason}
            status={data.status}
            workspaceId={data.id}
          />
        </div>
      ) : null}
    </section>
  );
}

function WorkspaceLifecycleActions({
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
  reason,
  status,
  workspaceId,
  confirmCode,
}: {
  activeAction: WorkspaceAction | null;
  actionResult: WorkspaceLifecycleResult | null;
  availableAction: WorkspaceAction | null;
  confirmCode: string;
  disabled: boolean;
  error: Error | null;
  isPending: boolean;
  onCancel: () => void;
  onConfirmCodeChange: (confirmCode: string) => void;
  onReasonChange: (reason: string) => void;
  onRun: (action: WorkspaceAction) => void;
  onStart: (action: WorkspaceAction) => void;
  reason: string;
  status: string;
  workspaceId: string;
}) {
  const isReasonReady = reason.trim().length >= 12;
  const actionLabel = availableAction === 'suspend' ? 'Suspendre' : 'Reactiver';
  const expectedCode = strongConfirmationCode(
    activeAction === 'suspend' ? 'SUSPEND WORKSPACE' : 'REACTIVATE WORKSPACE',
    workspaceId,
  );
  const isConfirmationReady = confirmCode.trim() === expectedCode;

  return (
    <div className="rounded-md border p-3">
      <div className="mb-3 flex flex-col gap-3 md:flex-row md:items-center md:justify-between">
        <div className="min-w-0">
          <div className="flex items-center gap-2 text-sm font-medium">
            <ShieldAlert className="text-muted-foreground size-4" />
            Cycle de vie workspace
          </div>
          <p className="text-muted-foreground mt-1 text-xs">
            Statut courant: <span className="font-medium">{status}</span>
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
          {actionResult.audit_action}: {actionResult.previous_status} vers{' '}
          {actionResult.next_status}
        </p>
      ) : null}
    </div>
  );
}

function WorkspaceMetric({
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
