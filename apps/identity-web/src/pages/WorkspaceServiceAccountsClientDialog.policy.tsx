import { Checkbox } from '@/components/ui/checkbox';
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
        <Checkbox
          id="client-assertion-required"
          checked={clientAssertionRequired}
          onCheckedChange={(checked) => setClientAssertionRequired(checked === true)}
        />
        <Label htmlFor="client-assertion-required">Client assertion requise</Label>
      </div>

      <ClientTextarea
        id="client-jwk"
        label="JWK public optionnel"
        value={clientAssertionJwk}
        onChange={setClientAssertionJwk}
        placeholder='{"kty":"RSA",...}'
        className="min-h-28 font-mono text-xs"
      />

      <DialogError message={clientError} />
    </>
  );
}
