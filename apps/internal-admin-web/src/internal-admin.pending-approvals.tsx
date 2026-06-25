import { useQuery } from '@tanstack/react-query';
import { AlertTriangle, CheckCircle2, Clock3, ExternalLink, ShieldCheck } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import { getPendingApprovals } from './internal-admin.api';
import { LockedState } from './internal-admin.locked-state';
import { PendingApprovalActions } from './internal-admin.pending-approval-actions';
import type { AdminCredentials, PendingApprovalItem } from './internal-admin.types';

export function PendingApprovalsPanel({
  credentials,
  disabled,
  onSelectTenant,
  onSelectUser,
  onSelectWorkspace,
}: {
  credentials: AdminCredentials;
  disabled: boolean;
  onSelectTenant: (tenantId: string) => void;
  onSelectUser: (principalId: string) => void;
  onSelectWorkspace: (workspaceId: string) => void;
}) {
  const approvals = useQuery({
    queryKey: ['pending-approvals'],
    queryFn: () => getPendingApprovals(credentials),
    enabled: !disabled,
  });
  const data = approvals.data;
  const items = data?.items ?? [];

  return (
    <section className="border-border bg-card mb-5 rounded-lg border p-4" id="pending-approvals">
      <div className="mb-4 flex flex-col gap-2 md:flex-row md:items-end md:justify-between">
        <div>
          <p className="text-muted-foreground text-xs font-medium tracking-[0.18em] uppercase">
            Dual-control queue
          </p>
          <h2 className="text-base font-semibold">Pending approvals</h2>
        </div>
        <p className="text-muted-foreground text-xs">
          Actions sensibles a traiter par role et contexte objet.
        </p>
      </div>
      {disabled ? (
        <LockedState label="Connecte un contexte operateur pour charger la file." />
      ) : null}
      <div className="mb-4 grid gap-3 sm:grid-cols-3">
        <QueueMetric icon={Clock3} label="Pending" value={formatCount(data?.pending_count)} />
        <QueueMetric
          icon={ShieldCheck}
          label="Critical"
          tone={(data?.critical_count ?? 0) > 0 ? 'warning' : 'default'}
          value={formatCount(data?.critical_count)}
        />
        <QueueMetric
          icon={AlertTriangle}
          label="Overdue"
          tone={(data?.overdue_count ?? 0) > 0 ? 'danger' : 'default'}
          value={formatCount(data?.overdue_count)}
        />
      </div>
      {items.length === 0 ? (
        <div className="bg-muted/30 rounded-md border p-4 text-sm">
          <div className="flex items-center gap-2 font-medium">
            <CheckCircle2 className="text-primary size-4" />
            Aucune approbation en attente
          </div>
          <p className="text-muted-foreground mt-1 text-xs">
            Les actions critiques apparaitront ici avec leur role requis, cible et lien direct.
          </p>
        </div>
      ) : (
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Action</TableHead>
              <TableHead>Objet</TableHead>
              <TableHead>Role requis</TableHead>
              <TableHead>Requested</TableHead>
              <TableHead>Links</TableHead>
              <TableHead>Decision</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {items.map((item) => (
              <TableRow key={item.id}>
                <TableCell>
                  <div className="font-medium">{item.action}</div>
                  <div className="text-muted-foreground text-xs">
                    {item.center} - {item.audit_hint}
                  </div>
                </TableCell>
                <TableCell>
                  <div>{item.target_label}</div>
                  <div className="text-muted-foreground text-xs">
                    {item.target_type} - {shortId(item.target_id)}
                  </div>
                </TableCell>
                <TableCell>
                  <Badge variant={item.severity === 'critical' ? 'destructive' : 'secondary'}>
                    {item.required_role}
                  </Badge>
                </TableCell>
                <TableCell>
                  <div>{formatDate(item.requested_at)}</div>
                  <div className="text-muted-foreground text-xs">
                    expire {formatDate(item.expires_at)}
                  </div>
                </TableCell>
                <TableCell>
                  <ApprovalLinks
                    item={item}
                    onSelectTenant={onSelectTenant}
                    onSelectUser={onSelectUser}
                    onSelectWorkspace={onSelectWorkspace}
                  />
                </TableCell>
                <TableCell>
                  <PendingApprovalActions credentials={credentials} item={item} />
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      )}
    </section>
  );
}

function QueueMetric({
  icon: Icon,
  label,
  tone = 'default',
  value,
}: {
  icon: typeof Clock3;
  label: string;
  tone?: 'danger' | 'default' | 'warning';
  value: string;
}) {
  const toneClass =
    tone === 'danger' ? 'text-destructive' : tone === 'warning' ? 'text-amber-600' : '';
  return (
    <div className="bg-muted/30 rounded-md border p-3">
      <div className="text-muted-foreground mb-3 flex items-center justify-between text-xs">
        {label}
        <Icon className={`size-4 ${toneClass}`} />
      </div>
      <div className={`text-xl font-semibold ${toneClass}`}>{value}</div>
    </div>
  );
}

function ApprovalLinks({
  item,
  onSelectTenant,
  onSelectUser,
  onSelectWorkspace,
}: {
  item: PendingApprovalItem;
  onSelectTenant: (tenantId: string) => void;
  onSelectUser: (principalId: string) => void;
  onSelectWorkspace: (workspaceId: string) => void;
}) {
  const tenantId = item.tenant_id;
  const workspaceId = item.workspace_id;
  const principalId = item.principal_id;

  return (
    <div className="flex flex-wrap gap-2">
      {tenantId ? (
        <Button onClick={() => openTenant(tenantId, onSelectTenant)} size="sm" variant="outline">
          <ExternalLink className="size-3.5" />
          Tenant
        </Button>
      ) : null}
      {workspaceId ? (
        <Button
          onClick={() => openWorkspace(workspaceId, onSelectWorkspace)}
          size="sm"
          variant="outline"
        >
          <ExternalLink className="size-3.5" />
          Workspace
        </Button>
      ) : null}
      {principalId ? (
        <Button onClick={() => openUser(principalId, onSelectUser)} size="sm" variant="outline">
          <ExternalLink className="size-3.5" />
          User
        </Button>
      ) : null}
    </div>
  );
}

function openTenant(tenantId: string, onSelectTenant: (tenantId: string) => void) {
  onSelectTenant(tenantId);
  window.location.hash = 'tenant-detail';
}

function openWorkspace(workspaceId: string, onSelectWorkspace: (workspaceId: string) => void) {
  onSelectWorkspace(workspaceId);
  window.location.hash = 'workspace-detail';
}

function openUser(principalId: string, onSelectUser: (principalId: string) => void) {
  onSelectUser(principalId);
  window.location.hash = 'user-detail';
}

function formatCount(value: number | undefined): string {
  return typeof value === 'number' ? new Intl.NumberFormat('fr-FR').format(value) : '-';
}

function formatDate(value: string | null): string {
  if (!value) return '-';
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return date.toLocaleString('fr-FR', {
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    month: '2-digit',
  });
}

function shortId(value: string): string {
  return value.length > 8 ? value.slice(0, 8) : value;
}
