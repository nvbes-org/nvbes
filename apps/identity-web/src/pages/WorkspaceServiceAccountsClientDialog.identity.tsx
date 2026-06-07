import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import type { WorkspaceServiceAccountsClientDialogModel } from './WorkspaceServiceAccountsClientDialog.types';
import { ClientTextarea } from './WorkspaceServiceAccountsClientDialog.textarea';

export function WorkspaceServiceAccountsClientIdentityFields({
  clientName,
  setClientName,
  clientScopes,
  setClientScopes,
  clientAudiences,
  setClientAudiences,
  clientResources,
  setClientResources,
}: Pick<
  WorkspaceServiceAccountsClientDialogModel,
  | 'clientName'
  | 'setClientName'
  | 'clientScopes'
  | 'setClientScopes'
  | 'clientAudiences'
  | 'setClientAudiences'
  | 'clientResources'
  | 'setClientResources'
>) {
  return (
    <>
      <div className="grid gap-2">
        <Label htmlFor="client-name">Nom</Label>
        <Input
          id="client-name"
          value={clientName}
          onChange={(event) => setClientName(event.target.value)}
          placeholder="drive-automation"
        />
      </div>

      <ClientTextarea
        id="client-scopes"
        label="Scopes autorises"
        value={clientScopes}
        onChange={setClientScopes}
        className="flex min-h-24 w-full rounded-lg border border-input bg-transparent px-3 py-2 text-sm shadow-xs outline-none transition-[color,box-shadow] placeholder:text-muted-foreground focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50"
      />
      <p className="-mt-2 text-xs text-muted-foreground">
        Separer les scopes par virgule ou nouvelle ligne. Le preset Drive est deja pre-rempli.
      </p>

      <div className="grid gap-2 sm:grid-cols-2">
        <ClientTextarea
          id="client-audiences"
          label="Audiences"
          value={clientAudiences}
          onChange={setClientAudiences}
        />
        <ClientTextarea
          id="client-resources"
          label="Resources"
          value={clientResources}
          onChange={setClientResources}
        />
      </div>
    </>
  );
}
