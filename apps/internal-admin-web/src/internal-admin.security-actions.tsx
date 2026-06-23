import { useMutation, useQueryClient } from '@tanstack/react-query';
import { RotateCcw } from 'lucide-react';
import { useState } from 'react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Textarea } from '@/components/ui/textarea';
import { revokeMfaFactor, revokeOauthConsent } from './internal-admin.api';
import type {
  ActiveMfaFactor,
  ActiveOauthConsent,
  AdminCredentials,
  SecurityActionResult,
} from './internal-admin.types';

type SecurityActionTarget =
  | { id: string; kind: 'mfa'; label: string }
  | { id: string; kind: 'oauth'; label: string };

export function SecurityActionSections({
  credentials,
  disabled,
  mfaFactors,
  oauthConsents,
  onSelectUser,
}: {
  credentials: AdminCredentials;
  disabled: boolean;
  mfaFactors: ActiveMfaFactor[];
  oauthConsents: ActiveOauthConsent[];
  onSelectUser: (principalId: string) => void;
}) {
  const queryClient = useQueryClient();
  const [activeTarget, setActiveTarget] = useState<SecurityActionTarget | null>(null);
  const [reason, setReason] = useState('');
  const [actionResult, setActionResult] = useState<SecurityActionResult | null>(null);
  const mutation = useMutation({
    mutationFn: async (target: SecurityActionTarget) =>
      target.kind === 'mfa'
        ? revokeMfaFactor(credentials, target.id, { reason })
        : revokeOauthConsent(credentials, target.id, { reason }),
    onSuccess: async (result) => {
      setActionResult(result);
      setActiveTarget(null);
      setReason('');
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ['security-center'] }),
        queryClient.invalidateQueries({ queryKey: ['internal-audit-events'] }),
        queryClient.invalidateQueries({ queryKey: ['user-detail', result.principal_id] }),
      ]);
    },
  });

  return (
    <>
      <div className="mt-4 grid gap-3 xl:grid-cols-2">
        <ActiveMfaFactorList
          activeTarget={activeTarget}
          disabled={disabled}
          error={mutation.error}
          isPending={mutation.isPending}
          onCancel={() => {
            setActiveTarget(null);
            setReason('');
          }}
          onReasonChange={setReason}
          onRevoke={(target) => mutation.mutate(target)}
          onSelectUser={onSelectUser}
          onStart={(target) => {
            setActionResult(null);
            setActiveTarget(target);
          }}
          reason={reason}
          rows={mfaFactors}
        />
        <ActiveOauthConsentList
          activeTarget={activeTarget}
          disabled={disabled}
          error={mutation.error}
          isPending={mutation.isPending}
          onCancel={() => {
            setActiveTarget(null);
            setReason('');
          }}
          onReasonChange={setReason}
          onRevoke={(target) => mutation.mutate(target)}
          onSelectUser={onSelectUser}
          onStart={(target) => {
            setActionResult(null);
            setActiveTarget(target);
          }}
          reason={reason}
          rows={oauthConsents}
        />
      </div>
      {actionResult ? (
        <div className="border-primary/20 bg-primary/5 text-primary mt-4 rounded-md border p-3 text-xs">
          {actionResult.audit_action}: {actionResult.object_id}
        </div>
      ) : null}
    </>
  );
}

function ActiveMfaFactorList({
  activeTarget,
  disabled,
  error,
  isPending,
  onCancel,
  onReasonChange,
  onRevoke,
  onSelectUser,
  onStart,
  reason,
  rows,
}: ActionListProps<ActiveMfaFactor>) {
  return (
    <div className="rounded-md border">
      <div className="border-b p-3">
        <h3 className="text-sm font-medium">MFA actifs</h3>
      </div>
      <div className="divide-y">
        {rows.length === 0 ? <EmptyRow label="Aucun facteur MFA actif recent." /> : null}
        {rows.map((row) => {
          const target: SecurityActionTarget = { id: row.id, kind: 'mfa', label: row.email };
          return (
            <article className="p-3" key={row.id}>
              <div className="mb-2 flex items-center justify-between gap-2">
                <div className="min-w-0">
                  <p className="truncate text-sm font-medium">{row.email}</p>
                  <p className="text-muted-foreground text-xs">
                    {row.factor_type}
                    {row.label ? ` - ${row.label}` : ''}
                  </p>
                </div>
                <Badge variant="secondary">{formatDate(row.last_used_at ?? row.created_at)}</Badge>
              </div>
              <SecurityActionControls
                activeTarget={activeTarget}
                disabled={disabled}
                error={error}
                isPending={isPending}
                onCancel={onCancel}
                onReasonChange={onReasonChange}
                onRevoke={onRevoke}
                onSelectUser={() => onSelectUser(row.principal_id)}
                onStart={onStart}
                reason={reason}
                target={target}
              />
            </article>
          );
        })}
      </div>
    </div>
  );
}

function ActiveOauthConsentList({
  activeTarget,
  disabled,
  error,
  isPending,
  onCancel,
  onReasonChange,
  onRevoke,
  onSelectUser,
  onStart,
  reason,
  rows,
}: ActionListProps<ActiveOauthConsent>) {
  return (
    <div className="rounded-md border">
      <div className="border-b p-3">
        <h3 className="text-sm font-medium">Consentements OAuth actifs</h3>
      </div>
      <div className="divide-y">
        {rows.length === 0 ? <EmptyRow label="Aucun consentement OAuth actif recent." /> : null}
        {rows.map((row) => {
          const target: SecurityActionTarget = { id: row.id, kind: 'oauth', label: row.email };
          return (
            <article className="p-3" key={row.id}>
              <div className="mb-2 flex items-center justify-between gap-2">
                <div className="min-w-0">
                  <p className="truncate text-sm font-medium">{row.email}</p>
                  <p className="text-muted-foreground truncate text-xs">
                    {row.workspace_name ?? row.tenant_name} - {row.scopes.join(', ') || 'no scope'}
                  </p>
                </div>
                <Badge variant="secondary">{formatDate(row.granted_at)}</Badge>
              </div>
              <SecurityActionControls
                activeTarget={activeTarget}
                disabled={disabled}
                error={error}
                isPending={isPending}
                onCancel={onCancel}
                onReasonChange={onReasonChange}
                onRevoke={onRevoke}
                onSelectUser={() => onSelectUser(row.principal_id)}
                onStart={onStart}
                reason={reason}
                target={target}
              />
            </article>
          );
        })}
      </div>
    </div>
  );
}

type ActionListProps<TRow> = {
  activeTarget: SecurityActionTarget | null;
  disabled: boolean;
  error: Error | null;
  isPending: boolean;
  onCancel: () => void;
  onReasonChange: (reason: string) => void;
  onRevoke: (target: SecurityActionTarget) => void;
  onSelectUser: (principalId: string) => void;
  onStart: (target: SecurityActionTarget) => void;
  reason: string;
  rows: TRow[];
};

function SecurityActionControls({
  activeTarget,
  disabled,
  error,
  isPending,
  onCancel,
  onReasonChange,
  onRevoke,
  onSelectUser,
  onStart,
  reason,
  target,
}: Omit<ActionListProps<unknown>, 'rows' | 'onSelectUser'> & {
  onSelectUser: () => void;
  target: SecurityActionTarget;
}) {
  const isActive = activeTarget?.id === target.id && activeTarget.kind === target.kind;
  const isReasonReady = reason.trim().length >= 12;

  return (
    <div>
      <div className="flex flex-wrap gap-2">
        <Button
          onClick={() => {
            onSelectUser();
            window.location.hash = 'user-detail';
          }}
          size="sm"
          type="button"
          variant="outline"
        >
          Open user
        </Button>
        <Button
          disabled={disabled || isPending}
          onClick={() => onStart(target)}
          size="sm"
          type="button"
          variant="destructive"
        >
          Revoquer
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
          <div className="flex flex-wrap items-center justify-between gap-2">
            <p className="text-muted-foreground text-xs">Minimum 12 caracteres requis.</p>
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
                disabled={disabled || !isReasonReady || isPending}
                onClick={() => onRevoke(target)}
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
    </div>
  );
}

function EmptyRow({ label }: { label: string }) {
  return <div className="text-muted-foreground p-4 text-sm">{label}</div>;
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
