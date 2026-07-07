import type { WorkspaceServiceAccountsClientDialogModel } from './WorkspaceServiceAccountsClientDialog.types';
import { WorkspaceServiceAccountsClientIdentityFields } from './WorkspaceServiceAccountsClientDialog.identity';
import { WorkspaceServiceAccountsClientPolicyFields } from './WorkspaceServiceAccountsClientDialog.policy';

export function WorkspaceServiceAccountsClientDialogFields(
  props: Pick<
    WorkspaceServiceAccountsClientDialogModel,
    | 'clientName'
    | 'setClientName'
    | 'clientScopes'
    | 'setClientScopes'
    | 'clientAudiences'
    | 'setClientAudiences'
    | 'clientResources'
    | 'setClientResources'
    | 'clientRequiredAcr'
    | 'setClientRequiredAcr'
    | 'clientAssertionRequired'
    | 'setClientAssertionRequired'
    | 'clientAssertionJwk'
    | 'setClientAssertionJwk'
    | 'clientError'
  >,
) {
  return (
    <div className="grid gap-4 py-4">
      <WorkspaceServiceAccountsClientIdentityFields {...props} />
      <WorkspaceServiceAccountsClientPolicyFields {...props} />
    </div>
  );
}
