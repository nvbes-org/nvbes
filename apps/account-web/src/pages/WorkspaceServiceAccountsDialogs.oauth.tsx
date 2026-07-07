import { WorkspaceServiceAccountsClientDialog } from './WorkspaceServiceAccountsClientDialog';
import type { WorkspaceServiceAccountsDialogsProps } from './WorkspaceServiceAccountsDialogs.types';

export function WorkspaceServiceAccountsOAuthDialogs({
  clientDialogOpen,
  setClientDialogOpen,
  clientName,
  setClientName,
  clientScopes,
  setClientScopes,
  clientAudiences,
  setClientAudiences,
  clientResources,
  setClientResources,
  clientRequiredAcr,
  setClientRequiredAcr,
  clientAssertionRequired,
  setClientAssertionRequired,
  clientAssertionJwk,
  setClientAssertionJwk,
  clientError,
  busyAction,
  onCreateClient,
  onClearClient,
}: Pick<
  WorkspaceServiceAccountsDialogsProps,
  | 'clientDialogOpen'
  | 'setClientDialogOpen'
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
  | 'busyAction'
  | 'onCreateClient'
  | 'onClearClient'
>) {
  return (
    <WorkspaceServiceAccountsClientDialog
      clientDialogOpen={clientDialogOpen}
      setClientDialogOpen={setClientDialogOpen}
      clientName={clientName}
      setClientName={setClientName}
      clientScopes={clientScopes}
      setClientScopes={setClientScopes}
      clientAudiences={clientAudiences}
      setClientAudiences={setClientAudiences}
      clientResources={clientResources}
      setClientResources={setClientResources}
      clientRequiredAcr={clientRequiredAcr}
      setClientRequiredAcr={setClientRequiredAcr}
      clientAssertionRequired={clientAssertionRequired}
      setClientAssertionRequired={setClientAssertionRequired}
      clientAssertionJwk={clientAssertionJwk}
      setClientAssertionJwk={setClientAssertionJwk}
      clientError={clientError}
      busyAction={busyAction}
      onCreateClient={onCreateClient}
      onClearClient={onClearClient}
    />
  );
}
