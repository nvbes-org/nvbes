import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { DialogError } from './WorkspaceServiceAccountsDialogs.shared';
import { ClientTextarea } from './WorkspaceServiceAccountsClientDialog.textarea';
import type { WorkspaceServiceAccountsClientDialogModel } from './WorkspaceServiceAccountsClientDialog.types';

export function WorkspaceServiceAccountsClientPolicyFields({
  clientRequiredAcr,
  setClientRequiredAcr,
  clientAssertionRequired,
  setClientAssertionRequired,
  clientAssertionJwk,
  setClientAssertionJwk,
  clientError,
}: Pick<
  WorkspaceServiceAccountsClientDialogModel,
  | 'clientRequiredAcr'
  | 'setClientRequiredAcr'
  | 'clientAssertionRequired'
  | 'setClientAssertionRequired'
  | 'clientAssertionJwk'
  | 'setClientAssertionJwk'
  | 'clientError'
>) {
  return (
    <>
      <div className="grid gap-2">
        <Label htmlFor="client-required-acr">Required ACR</Label>
        <Input
          id="client-required-acr"
          value={clientRequiredAcr}
          onChange={(event) => setClientRequiredAcr(event.target.value)}
          placeholder="aal2"
        />
      </div>

      <div className="flex items-center gap-2">
        <input
          id="client-assertion-required"
          type="checkbox"
          className="size-4 rounded border-border text-primary focus:ring-ring"
          checked={clientAssertionRequired}
          onChange={(event) => setClientAssertionRequired(event.target.checked)}
        />
        <Label htmlFor="client-assertion-required">Client assertion requise</Label>
      </div>

      <ClientTextarea
        id="client-jwk"
        label="JWK public optionnel"
        value={clientAssertionJwk}
        onChange={setClientAssertionJwk}
        placeholder='{"kty":"RSA",...}'
        className="flex min-h-28 w-full rounded-lg border border-input bg-transparent px-3 py-2 font-mono text-xs shadow-xs outline-none transition-[color,box-shadow] placeholder:text-muted-foreground focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50"
      />

      <DialogError message={clientError} />
    </>
  );
}
