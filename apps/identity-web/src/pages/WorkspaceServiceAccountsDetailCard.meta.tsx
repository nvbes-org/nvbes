import { UserCog } from 'lucide-react';
import { Card } from '@/components/ui/card';
import { Empty, EmptyDescription, EmptyHeader, EmptyMedia, EmptyTitle } from '@/components/ui/empty';

export function EmptyState() {
  return (
    <Empty className="border">
      <EmptyHeader>
        <EmptyMedia variant="icon">
          <UserCog />
        </EmptyMedia>
        <EmptyTitle>Aucun principal selectionne</EmptyTitle>
        <EmptyDescription>
          Choisissez un service account dans la liste pour voir ses details.
        </EmptyDescription>
      </EmptyHeader>
    </Empty>
  );
}

function MetaCard({ label, value }: { label: string; value: string }) {
  return (
    <Card className="p-4">
      <p className="text-xs uppercase tracking-[0.16em] text-muted-foreground">{label}</p>
      <p className="mt-2 break-all text-sm font-medium">{value}</p>
    </Card>
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
