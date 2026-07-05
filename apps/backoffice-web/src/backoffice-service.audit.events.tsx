import { useQuery } from '@tanstack/react-query';
import { Download, RefreshCw, Search, ShieldCheck } from 'lucide-react';
import { useMemo, useState } from 'react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Skeleton } from '@/components/ui/skeleton';
import { listAuditEvents } from './backoffice-service.api';
import { downloadEvidenceExport } from './backoffice-service.audit-evidence-download';
import { AuditEventRow } from './backoffice-service.audit-event-row';
import { sortAuditEvents } from './backoffice-service.audit-sort';
import {
  allAuditFilterValue,
  emptyAuditFilters,
  loadSavedAuditViews,
  removeAuditView,
  saveAuditViews,
  type AuditViewFilters,
  type SavedAuditView,
  upsertAuditView,
} from './backoffice-service.audit-view-state';
import { LockedState } from './backoffice-service.locked-state';
import type { AdminCredentials } from './backoffice-service.types';

const targetTypeOptions = ['tenant', 'workspace', 'principal', 'runbook', 'billing_invoice'];
const actionOptions = [
  'internal_admin.tenant.suspend',
  'internal_admin.tenant.reactivate',
  'internal_admin.workspace.suspend',
  'internal_admin.workspace.reactivate',
  'internal_admin.user.suspend',
  'internal_admin.user.reactivate',
  'internal_admin.runbook.executed',
];

export function AuditEventsPanel({
  credentials,
  disabled,
  onSelectTenant,
  onSelectUser,
  onSelectWorkspace,
}: {
  credentials: AdminCredentials;
  disabled: boolean;
  onSelectTenant?: (tenantId: string) => void;
  onSelectUser?: (principalId: string) => void;
  onSelectWorkspace?: (workspaceId: string) => void;
}) {
  const [action, setAction] = useState(emptyAuditFilters.action);
  const [sort, setSort] = useState(emptyAuditFilters.sort);
  const [targetType, setTargetType] = useState(emptyAuditFilters.targetType);
  const [query, setQuery] = useState(emptyAuditFilters.query);
  const [viewName, setViewName] = useState('');
  const [savedViews, setSavedViews] = useState<SavedAuditView[]>(loadSavedAuditViews);
  const filters = useMemo(
    () => ({
      action: action === allAuditFilterValue ? undefined : action,
      limit: 50,
      query,
      targetType: targetType === allAuditFilterValue ? undefined : targetType,
    }),
    [action, query, targetType],
  );
  const auditEvents = useQuery({
    queryKey: ['internal-audit-events', credentials.workspaceId, filters],
    queryFn: () => listAuditEvents(credentials, filters),
    enabled: !disabled,
  });
  const sortedAuditEvents = useMemo(
    () => sortAuditEvents(auditEvents.data ?? [], sort),
    [auditEvents.data, sort],
  );
  const [exportError, setExportError] = useState<string | null>(null);
  const [isExporting, setIsExporting] = useState(false);

  return (
    <section className="border-border bg-card rounded-lg border p-4" id="audit">
      <div className="mb-4 flex items-start justify-between gap-3">
        <div className="flex items-center gap-3">
          <div className="bg-primary/10 text-primary flex size-9 items-center justify-center rounded-md">
            <ShieldCheck className="size-4" />
          </div>
          <div>
            <h2 className="text-sm font-semibold">Audit recent</h2>
            <p className="text-muted-foreground text-xs">
              Derniers evenements append-only du tenant courant.
            </p>
          </div>
        </div>
        <div className="flex flex-wrap gap-2">
          <Button
            disabled={disabled || isExporting}
            onClick={() => {
              void downloadEvidenceExport(credentials, filters, {
                setError: setExportError,
                setIsExporting,
              });
            }}
            size="sm"
            type="button"
            variant="outline"
          >
            <Download className="size-4" />
            Export evidence
          </Button>
          <Button
            disabled={disabled || auditEvents.isFetching}
            onClick={() => void auditEvents.refetch()}
            size="sm"
            type="button"
            variant="outline"
          >
            <RefreshCw className="size-4" />
            Refresh
          </Button>
        </div>
      </div>

      <div className="mb-3 grid gap-2 lg:grid-cols-[1fr_auto_auto_auto_auto]">
        <div className="relative">
          <Search className="text-muted-foreground pointer-events-none absolute top-2 left-2 size-4" />
          <Input
            className="pl-8"
            disabled={disabled}
            onChange={(event) => setQuery(event.target.value)}
            placeholder="Search action, actor, target..."
            value={query}
          />
        </div>
        <Select disabled={disabled} onValueChange={setAction} value={action}>
          <SelectTrigger className="w-full lg:w-72">
            <SelectValue placeholder="Action" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value={allAuditFilterValue}>Toutes les actions</SelectItem>
            {actionOptions.map((item) => (
              <SelectItem key={item} value={item}>
                {item}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <Select disabled={disabled} onValueChange={setTargetType} value={targetType}>
          <SelectTrigger className="w-full lg:w-44">
            <SelectValue placeholder="Target" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value={allAuditFilterValue}>Toutes les cibles</SelectItem>
            {targetTypeOptions.map((item) => (
              <SelectItem key={item} value={item}>
                {item}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <Select
          disabled={disabled}
          onValueChange={(value) => setSort(value as typeof sort)}
          value={sort}
        >
          <SelectTrigger className="w-full lg:w-44">
            <SelectValue placeholder="Sort" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="newest">Recent first</SelectItem>
            <SelectItem value="oldest">Oldest first</SelectItem>
            <SelectItem value="hash_anomalies">Hash issues</SelectItem>
            <SelectItem value="sensitive_first">Sensitive first</SelectItem>
          </SelectContent>
        </Select>
        <Button
          disabled={disabled || auditEvents.isFetching}
          onClick={() => {
            applyAuditFilters(emptyAuditFilters, { setAction, setQuery, setSort, setTargetType });
          }}
          type="button"
          variant="outline"
        >
          Reset
        </Button>
      </div>
      <div className="mb-4 grid gap-2 lg:grid-cols-[1fr_auto_auto_auto]">
        <Input
          disabled={disabled}
          onChange={(event) => setViewName(event.target.value)}
          placeholder="Nom de vue sauvegardee"
          value={viewName}
        />
        <Button
          disabled={disabled || !viewName.trim()}
          onClick={() => {
            const nextViews = upsertAuditView(savedViews, viewName, {
              action,
              query,
              sort,
              targetType,
            });
            setSavedViews(nextViews);
            saveAuditViews(nextViews);
            setViewName('');
          }}
          type="button"
          variant="outline"
        >
          Save view
        </Button>
        <Select
          disabled={disabled || savedViews.length === 0}
          onValueChange={(viewId) => {
            const view = savedViews.find((item) => item.id === viewId);
            if (view) applyAuditFilters(view, { setAction, setQuery, setSort, setTargetType });
          }}
          value=""
        >
          <SelectTrigger className="w-full lg:w-60">
            <SelectValue placeholder="Vues sauvegardees" />
          </SelectTrigger>
          <SelectContent>
            {savedViews.map((view) => (
              <SelectItem key={view.id} value={view.id}>
                {view.name}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <Button
          disabled={disabled || savedViews.length === 0}
          onClick={() => {
            const [firstView] = savedViews;
            if (!firstView) return;
            const nextViews = removeAuditView(savedViews, firstView.id);
            setSavedViews(nextViews);
            saveAuditViews(nextViews);
          }}
          type="button"
          variant="outline"
        >
          Delete latest
        </Button>
      </div>

      <div className="space-y-2">
        {disabled ? (
          <LockedState label="Connecte un contexte operateur pour consulter l'audit." />
        ) : null}
        {auditEvents.isLoading ? <Skeleton className="h-20 w-full" /> : null}
        {exportError ? (
          <p className="text-destructive rounded-md border p-3 text-sm">{exportError}</p>
        ) : null}
        {sortedAuditEvents.map((event) => (
          <AuditEventRow
            event={event}
            key={event.id}
            onSelectTenant={onSelectTenant}
            onSelectUser={onSelectUser}
            onSelectWorkspace={onSelectWorkspace}
          />
        ))}
        {auditEvents.data?.length === 0 ? (
          <p className="text-muted-foreground rounded-md border p-3 text-sm">
            Aucun evenement audit trouve pour ce tenant.
          </p>
        ) : null}
        {auditEvents.error ? (
          <p className="text-destructive rounded-md border p-3 text-sm">
            {auditEvents.error instanceof Error ? auditEvents.error.message : 'Audit unavailable'}
          </p>
        ) : null}
      </div>
    </section>
  );
}

type AuditFilterSetters = {
  setAction: (value: string) => void;
  setQuery: (value: string) => void;
  setSort: (value: AuditViewFilters['sort']) => void;
  setTargetType: (value: string) => void;
};

function applyAuditFilters(filters: AuditViewFilters, setters: AuditFilterSetters) {
  setters.setAction(filters.action);
  setters.setQuery(filters.query);
  setters.setSort(filters.sort);
  setters.setTargetType(filters.targetType);
}
