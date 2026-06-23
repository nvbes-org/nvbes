import { ClipboardButton } from '@nvbes/web-ui';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import {
  AlertTriangle,
  Building2,
  Clock3,
  Lock,
  RotateCcw,
  ShieldCheck,
  Users,
} from 'lucide-react';
import type { ComponentType } from 'react';
import { useState } from 'react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Textarea } from '@/components/ui/textarea';
import { getTenantDetail, reactivateTenant, suspendTenant } from './internal-admin.api';
import { LockedState } from './internal-admin.locked-state';
import type { AdminCredentials, TenantLifecycleResult } from './internal-admin.types';

type TenantAction = 'reactivate' | 'suspend';

export function TenantDetailPanel({
  credentials,
  disabled,
  tenantId,
}: {
  credentials: AdminCredentials;
  disabled: boolean;
  tenantId: string | null;
}) {
  const queryClient = useQueryClient();
  const [activeAction, setActiveAction] = useState<TenantAction | null>(null);
  const [confirmCode, setConfirmCode] = useState('');
  const [reason, setReason] = useState('');
  const [actionResult, setActionResult] = useState<TenantLifecycleResult | null>(null);
  const tenant = useQuery({
    queryKey: ['tenant-detail', tenantId],
    queryFn: () => getTenantDetail(credentials, tenantId ?? ''),
    enabled: !disabled && tenantId !== null,
  });
  const data = tenant.data;
  const mutation = useMutation({
    mutationFn: async (action: TenantAction) => {
      if (!tenantId) throw new Error('Tenant absent.');
      const body = { confirm_code: confirmCode, reason };
      return action === 'suspend'
        ? suspendTenant(credentials, tenantId, body)
        : reactivateTenant(credentials, tenantId, body);
    },
    onSuccess: async (result) => {
      setActionResult(result);
      setActiveAction(null);
      setConfirmCode('');
      setReason('');
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ['tenant-detail', tenantId] }),
        queryClient.invalidateQueries({ queryKey: ['command-center'] }),
        queryClient.invalidateQueries({ queryKey: ['customer-center'] }),
        queryClient.invalidateQueries({ queryKey: ['global-search'] }),
      ]);
    },
  });
  const availableAction =
    data?.status === 'active' ? 'suspend' : data?.status === 'suspended' ? 'reactivate' : null;

  return (
    <section className="border-border bg-card mb-5 rounded-lg border p-4" id="tenant-detail">
      <div className="mb-4 flex flex-col gap-2 md:flex-row md:items-start md:justify-between">
        <div>
          <p className="text-muted-foreground text-xs font-medium tracking-[0.18em] uppercase">
            Tenant detail
          </p>
          <h2 className="text-base font-semibold">
            {data ? data.name : tenantId ? 'Chargement tenant' : 'Aucun tenant selectionne'}
          </h2>
        </div>
        {data ? (
          <div className="flex flex-wrap gap-2">
            <Badge variant="secondary">{data.status}</Badge>
            <Badge variant="outline">{data.security_tier}</Badge>
          </div>
        ) : null}
      </div>
      {disabled ? (
        <LockedState label="Connecte un contexte operateur pour ouvrir une fiche tenant." />
      ) : null}
      {!disabled && !tenantId ? (
        <div className="bg-muted/30 rounded-md border p-4 text-sm">
          Selectionne un tenant dans Global Search pour ouvrir sa fiche.
        </div>
      ) : null}
      {tenant.error ? (
        <div className="border-destructive/20 bg-destructive/5 text-destructive rounded-md border p-3 text-sm">
          {tenant.error instanceof Error ? tenant.error.message : 'Tenant unavailable'}
        </div>
      ) : null}
      {data ? (
        <div className="space-y-4">
          <div className="flex items-center justify-between gap-3 rounded-md border p-3">
            <div className="min-w-0">
              <p className="text-sm font-medium">{data.slug}</p>
              <p className="text-muted-foreground truncate font-mono text-xs">{data.id}</p>
            </div>
            <ClipboardButton
              className="h-7 rounded-[min(var(--radius-md),12px)] text-[0.8rem]"
              label="Copy tenant ID"
              value={data.id}
            />
          </div>
          <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-5">
            <TenantMetric
              icon={Building2}
              label="Workspaces"
              value={formatCount(data.workspace_count)}
            />
            <TenantMetric icon={Users} label="Users" value={formatCount(data.user_count)} />
            <TenantMetric
              icon={Clock3}
              label="Audit 24h"
              value={formatCount(data.audit_events_24h)}
            />
            <TenantMetric
              icon={ShieldCheck}
              label="Open invoices"
              value={formatCount(data.open_invoice_count)}
            />
            <TenantMetric
              icon={AlertTriangle}
              label="Provider failures"
              tone={data.provider_failure_count > 0 ? 'danger' : 'default'}
              value={formatCount(data.provider_failure_count)}
            />
          </div>
          <TenantLifecycleActions
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
          />
        </div>
      ) : null}
    </section>
  );
}

function TenantMetric({
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

function TenantLifecycleActions({
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
  confirmCode,
}: {
  activeAction: TenantAction | null;
  actionResult: TenantLifecycleResult | null;
  availableAction: TenantAction | null;
  confirmCode: string;
  disabled: boolean;
  error: Error | null;
  isPending: boolean;
  onCancel: () => void;
  onConfirmCodeChange: (confirmCode: string) => void;
  onReasonChange: (reason: string) => void;
  onRun: (action: TenantAction) => void;
  onStart: (action: TenantAction) => void;
  reason: string;
  status: string;
}) {
  const actionLabel = availableAction === 'suspend' ? 'Suspendre' : 'Reactiver';
  const isReasonReady = reason.trim().length >= 12;
  const expectedCode = activeAction === 'suspend' ? 'SUSPEND TENANT' : 'REACTIVATE TENANT';
  const isConfirmationReady = confirmCode.trim() === expectedCode;

  return (
    <div className="rounded-md border p-3">
      <div className="mb-3 flex flex-col gap-3 md:flex-row md:items-center md:justify-between">
        <div className="min-w-0">
          <div className="flex items-center gap-2 text-sm font-medium">
            <Lock className="text-muted-foreground size-4" />
            Cycle de vie tenant
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
              <Lock className="size-4" />
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
            placeholder="Motif audit, ticket, approbation, impact..."
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

function formatCount(value: number): string {
  return new Intl.NumberFormat('fr-FR').format(value);
}
