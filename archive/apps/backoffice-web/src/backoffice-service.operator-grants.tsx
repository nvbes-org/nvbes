import { useMutation, useQueryClient } from '@tanstack/react-query';
import { ShieldCheck, UserRound } from 'lucide-react';
import { useState } from 'react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Textarea } from '@/components/ui/textarea';
import { grantOperatorRole, revokeOperatorRole } from './backoffice-service.api';
import { strongConfirmationCode } from './backoffice-service.strong-confirmation';
import type {
  AdminCredentials,
  BackofficeRole,
  OperatorGrant,
  OperatorGrantActionResult,
  OperatorRoleDistribution,
} from './backoffice-service.types';

type OperatorGrantAction =
  | { mode: 'grant'; principalId: string; role: BackofficeRole }
  | { mode: 'revoke'; principalId: string; role: BackofficeRole };

export function OperatorGrantsPanel({
  credentials,
  disabled,
  grants,
  roleDistribution,
}: {
  credentials: AdminCredentials;
  disabled: boolean;
  grants: OperatorGrant[];
  roleDistribution: OperatorRoleDistribution[];
}) {
  const queryClient = useQueryClient();
  const [activeAction, setActiveAction] = useState<OperatorGrantAction | null>(null);
  const [principalId, setPrincipalId] = useState('');
  const [role, setRole] = useState<BackofficeRole>('viewer');
  const [confirmCode, setConfirmCode] = useState('');
  const [reason, setReason] = useState('');
  const [result, setResult] = useState<OperatorGrantActionResult | null>(null);
  const mutation = useMutation({
    mutationFn: (action: OperatorGrantAction) => {
      const body = { confirm_code: confirmCode, reason };
      return action.mode === 'grant'
        ? grantOperatorRole(credentials, action.principalId, action.role, body)
        : revokeOperatorRole(credentials, action.principalId, action.role, body);
    },
    onSuccess: async (data) => {
      setResult(data);
      resetAction();
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ['identity-governance-center'] }),
        queryClient.invalidateQueries({ queryKey: ['internal-audit-events'] }),
      ]);
    },
  });
  const expectedCode = activeAction ? confirmationCode(activeAction) : '';
  const isActionReady =
    activeAction !== null && reason.trim().length >= 12 && confirmCode.trim() === expectedCode;

  return (
    <div className="rounded-md border" data-testid="operator-grants-panel">
      <div className="border-b p-3">
        <div className="flex items-center justify-between gap-3">
          <h3 className="text-sm font-medium">Back-office operator grants</h3>
          <Badge variant="outline">{grants.length} visible</Badge>
        </div>
        <div className="mt-3 flex flex-wrap gap-2">
          {roleDistribution.length === 0 ? (
            <Badge variant="secondary">No active grants</Badge>
          ) : (
            roleDistribution.map((row) => (
              <Badge key={row.role} variant="secondary">
                {row.role}: {row.active_count}
              </Badge>
            ))
          )}
        </div>
      </div>
      <div className="bg-muted/20 grid gap-3 border-b p-3">
        <div className="grid gap-2 md:grid-cols-[1fr_220px_auto]">
          <Input
            disabled={disabled || mutation.isPending}
            onChange={(event) => setPrincipalId(event.target.value)}
            placeholder="Principal UUID"
            value={principalId}
          />
          <Select
            disabled={disabled || mutation.isPending}
            onValueChange={(value) => setRole(value as BackofficeRole)}
            value={role}
          >
            <SelectTrigger className="w-full">
              <SelectValue placeholder="Role" />
            </SelectTrigger>
            <SelectContent>
              {backofficeRoles.map((candidate) => (
                <SelectItem key={candidate} value={candidate}>
                  {candidate}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <Button
            disabled={disabled || mutation.isPending || !principalId.trim()}
            onClick={() => {
              setResult(null);
              setActiveAction({ mode: 'grant', principalId: principalId.trim(), role });
              setConfirmCode('');
              setReason('');
            }}
            type="button"
          >
            Grant role
          </Button>
        </div>
        {activeAction ? (
          <div className="grid gap-2 rounded-md border bg-background p-3">
            <Textarea
              disabled={disabled || mutation.isPending}
              onChange={(event) => setReason(event.target.value)}
              placeholder="Motif audit, ticket, approbation..."
              value={reason}
            />
            <Input
              className="font-mono text-xs"
              disabled={disabled || mutation.isPending}
              onChange={(event) => setConfirmCode(event.target.value)}
              placeholder={expectedCode}
              value={confirmCode}
            />
            <div className="flex flex-wrap items-center justify-between gap-2">
              <p className="text-muted-foreground text-xs">Code exact requis: {expectedCode}</p>
              <div className="flex gap-2">
                <Button
                  disabled={mutation.isPending}
                  onClick={resetAction}
                  type="button"
                  variant="outline"
                >
                  Annuler
                </Button>
                <Button
                  disabled={disabled || mutation.isPending || !isActionReady}
                  onClick={() => mutation.mutate(activeAction)}
                  type="button"
                  variant={activeAction.mode === 'revoke' ? 'destructive' : 'default'}
                >
                  Confirmer
                </Button>
              </div>
            </div>
          </div>
        ) : null}
        {mutation.error ? (
          <p className="text-destructive text-xs">
            {mutation.error instanceof Error ? mutation.error.message : 'Action impossible'}
          </p>
        ) : null}
        {result ? (
          <p className="text-primary text-xs">
            {result.audit_action}: {result.role} {result.previous_status ?? 'none'} vers{' '}
            {result.next_status}
          </p>
        ) : null}
      </div>
      <div className="divide-y">
        {grants.length === 0 ? (
          <div className="text-muted-foreground p-4 text-sm">Aucun grant operateur configure.</div>
        ) : (
          grants.map((grant) => {
            const grantRole = toBackofficeRole(grant.role);
            return (
              <OperatorGrantRow
                disabled={disabled || mutation.isPending || grantRole === null}
                grant={grant}
                key={`${grant.principal_id}:${grant.role}`}
                onRevoke={() => {
                  if (grantRole === null) return;
                  setResult(null);
                  setActiveAction({
                    mode: 'revoke',
                    principalId: grant.principal_id,
                    role: grantRole,
                  });
                  setConfirmCode('');
                  setReason('');
                }}
              />
            );
          })
        )}
      </div>
    </div>
  );

  function resetAction() {
    setActiveAction(null);
    setConfirmCode('');
    setReason('');
  }
}

function OperatorGrantRow({
  disabled,
  grant,
  onRevoke,
}: {
  disabled: boolean;
  grant: OperatorGrant;
  onRevoke: () => void;
}) {
  const identity = grant.email ?? grant.display_name ?? grant.principal_id;
  const statusTone = grant.status === 'active' ? 'secondary' : 'outline';

  return (
    <article className="p-3">
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0">
          <div className="flex items-center gap-2">
            {grant.status === 'active' ? (
              <ShieldCheck className="text-primary size-4" />
            ) : (
              <UserRound className="text-muted-foreground size-4" />
            )}
            <p className="truncate text-sm font-medium">{identity}</p>
          </div>
          <p className="text-muted-foreground mt-1 truncate font-mono text-xs">
            {grant.principal_id}
          </p>
          {grant.reason ? (
            <p className="text-muted-foreground mt-1 text-xs">{grant.reason}</p>
          ) : null}
        </div>
        <div className="flex shrink-0 flex-col items-end gap-2">
          <Badge variant={statusTone}>{grant.status}</Badge>
          <Badge variant="outline">{grant.role}</Badge>
          {grant.status === 'active' ? (
            <Button
              disabled={disabled}
              onClick={onRevoke}
              size="sm"
              type="button"
              variant="destructive"
            >
              Revoke
            </Button>
          ) : null}
        </div>
      </div>
      <p className="text-muted-foreground mt-2 text-xs">
        Granted {formatDate(grant.granted_at)}
        {grant.revoked_at ? ` - revoked ${formatDate(grant.revoked_at)}` : ''}
      </p>
    </article>
  );
}

function confirmationCode(action: OperatorGrantAction): string {
  const verb = action.mode === 'grant' ? 'GRANT OPERATOR' : 'REVOKE OPERATOR';
  return strongConfirmationCode(`${verb} ${action.role.toUpperCase()}`, action.principalId);
}

function formatDate(value: string): string {
  return new Intl.DateTimeFormat('fr-FR').format(new Date(value));
}

const backofficeRoles: BackofficeRole[] = [
  'platform_admin',
  'security_admin',
  'finance_admin',
  'developer_admin',
  'operations_admin',
  'product_admin',
  'compliance_admin',
  'support_agent',
  'viewer',
];

function toBackofficeRole(value: string): BackofficeRole | null {
  return backofficeRoles.find((role) => role === value) ?? null;
}
