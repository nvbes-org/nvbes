import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import {
  WorkspaceServiceAccountsClientDialogFields,
  WorkspaceServiceAccountsClientDialogFooter,
  type WorkspaceServiceAccountsClientDialogModel,
} from './WorkspaceServiceAccountsClientDialog.shared';

export function WorkspaceServiceAccountsClientDialog({
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
}: WorkspaceServiceAccountsClientDialogModel) {
  const handleCancel = () => {
    setClientDialogOpen(false);
    onClearClient();
  };

  return (
    <Dialog
      open={clientDialogOpen}
      onOpenChange={(open) => {
        setClientDialogOpen(open);
        if (!open) onClearClient();
      }}
    >
      <DialogContent className="max-w-2xl">
        <DialogHeader>
          <DialogTitle>Creer un client OAuth</DialogTitle>
          <DialogDescription>
            Generation d&apos;un client attache au service account selectionne. Le secret est
            affiche une seule fois apres creation.
          </DialogDescription>
        </DialogHeader>
        <WorkspaceServiceAccountsClientDialogFields
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
        />
        <DialogFooter>
          <WorkspaceServiceAccountsClientDialogFooter
            busyAction={busyAction}
            onCancel={handleCancel}
            onCreateClient={onCreateClient}
          />
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
