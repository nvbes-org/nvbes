import { UserCog } from 'lucide-react';

export function EmptyState() {
  return (
    <div className="flex flex-col items-center gap-3 rounded-2xl border border-dashed border-border/70 px-6 py-10 text-center">
      <div className="flex size-12 items-center justify-center rounded-full bg-muted">
        <UserCog className="size-5 text-muted-foreground" />
      </div>
      <div className="flex flex-col gap-1">
        <p className="text-sm font-medium">Aucun principal selectionne</p>
        <p className="text-sm text-muted-foreground">
          Choisissez un service account dans la liste pour voir ses details.
        </p>
      </div>
    </div>
  );
}

function MetaCard({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-2xl border border-border/70 bg-muted/20 p-4">
      <p className="text-xs uppercase tracking-[0.16em] text-muted-foreground">{label}</p>
      <p className="mt-2 break-all text-sm font-medium">{value}</p>
    </div>
  );
}

export function ServiceAccountOverview({
  selectedServiceAccount,
}: {
  selectedServiceAccount: import('../identity.service-accounts.api').ServiceAccount;
}) {
  return (
    <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-4">
      <MetaCard label="Principal ID" value={selectedServiceAccount.principal_id} />
      <MetaCard label="Tenant" value={selectedServiceAccount.tenant_id} />
      <MetaCard label="Organization" value={selectedServiceAccount.organization_id ?? 'Aucune'} />
      <MetaCard label="Workspace" value={selectedServiceAccount.workspace_id} />
    </div>
  );
}
