import { Building } from 'lucide-react';
import type { AccountWorkspace } from '@nvbes/identity-client';
import { Badge } from '@/components/ui/badge';

export function WorkspaceRow({
  workspace,
  isCurrent,
}: {
  workspace: AccountWorkspace;
  isCurrent: boolean;
}) {
  return (
    <div className="flex items-center justify-between gap-3 py-1">
      <div className="flex min-w-0 items-center gap-3">
        <div className="flex size-8 shrink-0 items-center justify-center rounded-lg bg-muted">
          <Building className="size-4 text-muted-foreground" />
        </div>
        <div className="flex min-w-0 flex-col">
          <span className="text-sm font-medium">{workspace.name}</span>
          <span className="text-xs text-muted-foreground">
            {workspace.workspace_type} · {workspace.role}
            {workspace.plan_code ? ` · ${workspace.plan_code}` : ''}
          </span>
        </div>
      </div>
      <div className="flex items-center gap-2">
        {isCurrent ? (
          <Badge variant="default" className="shrink-0">
            Actuel
          </Badge>
        ) : null}
        <Badge variant="secondary" className="shrink-0">
          {workspace.role}
        </Badge>
      </div>
    </div>
  );
}
