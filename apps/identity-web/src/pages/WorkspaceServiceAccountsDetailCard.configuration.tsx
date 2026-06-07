import { Loader2, Shield } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import type { ServiceAccount } from '../identity.service-accounts.api';
import { badgeVariantForStatus, statusLabel } from './WorkspaceServiceAccounts.helpers';

export function ServiceAccountConfiguration({
  selectedServiceAccount,
  updateName,
  setUpdateName,
  updateDescription,
  setUpdateDescription,
  updateRole,
  setUpdateRole,
  busyAction,
  editError,
  onSave,
}: {
  selectedServiceAccount: ServiceAccount;
  updateName: string;
  setUpdateName: (value: string) => void;
  updateDescription: string;
  setUpdateDescription: (value: string) => void;
  updateRole: string;
  setUpdateRole: (value: string) => void;
  busyAction: string | null;
  editError: string | null;
  onSave: () => void;
}) {
  return (
    <div className="rounded-3xl border border-border/70 bg-background p-5">
      <div className="flex items-center justify-between gap-3">
        <div>
          <p className="text-sm font-medium">Configuration</p>
          <p className="text-sm text-muted-foreground">
            Mettre a jour le nom, la description et le role RBAC.
          </p>
        </div>
        <Badge variant={badgeVariantForStatus(selectedServiceAccount.status)}>
          {statusLabel(selectedServiceAccount.status)}
        </Badge>
      </div>

      <div className="mt-4 grid gap-4">
        <div className="grid gap-2">
          <label htmlFor="service-account-name" className="text-sm font-medium">
            Nom
          </label>
          <input
            id="service-account-name"
            className="flex h-10 w-full rounded-lg border border-input bg-transparent px-3 py-2 text-sm outline-none transition-[color,box-shadow] placeholder:text-muted-foreground focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50"
            value={updateName}
            onChange={(event) => setUpdateName(event.target.value)}
          />
        </div>

        <div className="grid gap-2">
          <label htmlFor="service-account-description" className="text-sm font-medium">
            Description
          </label>
          <textarea
            id="service-account-description"
            className="flex min-h-24 w-full rounded-lg border border-input bg-transparent px-3 py-2 text-sm shadow-xs outline-none transition-[color,box-shadow] placeholder:text-muted-foreground focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50"
            value={updateDescription}
            onChange={(event) => setUpdateDescription(event.target.value)}
          />
        </div>

        <div className="grid gap-2">
          <label htmlFor="service-account-role" className="text-sm font-medium">
            Role RBAC
          </label>
          <select
            id="service-account-role"
            className="flex h-10 w-full rounded-lg border border-input bg-transparent px-3 py-2 text-sm outline-none transition-[color,box-shadow] focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50"
            value={updateRole}
            onChange={(event) => setUpdateRole(event.target.value)}
          >
            <option value="viewer">viewer</option>
            <option value="member">member</option>
            <option value="admin">admin</option>
            <option value="owner">owner</option>
          </select>
        </div>

        {editError ? (
          <div className="rounded-lg border border-destructive/30 bg-destructive/5 px-3 py-2 text-sm text-destructive">
            {editError}
          </div>
        ) : null}
      </div>

      <div className="mt-4 flex flex-wrap gap-2">
        <Button onClick={onSave} disabled={busyAction === 'update-service-account'}>
          {busyAction === 'update-service-account' ? (
            <>
              <Loader2 className="size-4 animate-spin" />
              Sauvegarde...
            </>
          ) : (
            <>
              <Shield className="size-4" />
              Sauvegarder
            </>
          )}
        </Button>
      </div>
    </div>
  );
}
