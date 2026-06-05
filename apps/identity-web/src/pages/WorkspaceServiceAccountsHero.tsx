import type { AccountWorkspace } from '@nvbes/identity-client';
import { KeyRound, Plus, RefreshCw, ServerCog, ShieldCheck } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';

function MetricCard({
  label,
  value,
  hint,
  icon: Icon,
}: {
  label: string;
  value: string;
  hint: string;
  icon: typeof ServerCog;
}) {
  return (
    <div className="rounded-2xl border border-border/70 bg-background/80 p-4 shadow-sm">
      <div className="flex items-start justify-between gap-3">
        <div>
          <p className="text-xs uppercase tracking-[0.16em] text-muted-foreground">{label}</p>
          <p className="mt-2 text-2xl font-semibold">{value}</p>
          <p className="mt-1 text-xs text-muted-foreground">{hint}</p>
        </div>
        <div className="flex size-10 items-center justify-center rounded-xl bg-primary/10 text-primary">
          <Icon className="size-4" />
        </div>
      </div>
    </div>
  );
}

export function WorkspaceServiceAccountsHero({
  workspace,
  workspaceId,
  workspaces,
  canManage,
  serviceAccountsCount,
  activeCount,
  clientCount,
  onWorkspaceChange,
  onCreate,
  onRefresh,
}: {
  workspace: AccountWorkspace | null;
  workspaceId: string;
  workspaces: AccountWorkspace[];
  canManage: boolean;
  serviceAccountsCount: number;
  activeCount: number;
  clientCount: number;
  onWorkspaceChange: (value: string) => void;
  onCreate: () => void;
  onRefresh: () => void;
}) {
  return (
    <div className="relative overflow-hidden rounded-3xl border border-border/70 bg-gradient-to-br from-primary/10 via-background to-muted/50 p-6 shadow-sm">
      <div className="absolute inset-0 bg-[radial-gradient(circle_at_top_right,rgba(255,255,255,0.45),transparent_28%),linear-gradient(135deg,transparent_0%,transparent_65%,rgba(255,255,255,0.12)_100%)] opacity-70 dark:opacity-40" />
      <div className="relative flex flex-col gap-6 lg:flex-row lg:items-end lg:justify-between">
        <div className="max-w-2xl">
          <div className="flex flex-wrap items-center gap-2">
            <Badge variant="secondary">Admin workspace</Badge>
            {workspace ? <Badge variant="outline">{workspace.role}</Badge> : null}
          </div>
          <h1 className="mt-4 text-2xl font-semibold tracking-tight">Comptes de service</h1>
          <p className="mt-2 max-w-xl text-sm text-muted-foreground">
            Gerer les identites machine du workspace courant, leurs clients OAuth, leur role et leur
            cycle de vie. Les operations ici correspondent directement aux endpoints Identity, sans
            acces direct a la base de donnees.
          </p>
        </div>
        <div className="flex flex-col gap-3 sm:flex-row">
          <div className="w-full sm:w-72">
            <Label className="mb-2 block text-xs uppercase tracking-[0.16em] text-muted-foreground">
              Workspace
            </Label>
            <Select value={workspaceId} onValueChange={onWorkspaceChange}>
              <SelectTrigger className="w-full">
                <SelectValue placeholder="Choisir un workspace" />
              </SelectTrigger>
              <SelectContent>
                {workspaces.map((entry) => (
                  <SelectItem key={entry.id} value={entry.id}>
                    {entry.name}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
          <Button className="h-10" onClick={onCreate} disabled={!workspaceId || !canManage}>
            <Plus className="size-4" />
            Nouveau service account
          </Button>
          <Button variant="outline" className="h-10" onClick={onRefresh} disabled={!workspaceId}>
            <RefreshCw className="size-4" />
            Rafraichir
          </Button>
        </div>
      </div>

      <div className="relative mt-6 grid gap-3 sm:grid-cols-3">
        <MetricCard
          label="Service accounts"
          value={String(serviceAccountsCount)}
          hint="Identites machine rattachees au workspace."
          icon={ServerCog}
        />
        <MetricCard
          label="Actifs"
          value={String(activeCount)}
          hint="Prets a emettre des tokens M2M."
          icon={ShieldCheck}
        />
        <MetricCard
          label="Clients OAuth"
          value={String(clientCount)}
          hint="Clients attachés ou generes pour ces principals."
          icon={KeyRound}
        />
      </div>
    </div>
  );
}
