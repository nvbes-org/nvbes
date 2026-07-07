import { WorkspaceServiceAccountsAttachDialog } from './WorkspaceServiceAccountsAttachDialog';
import { SecretDialog } from './WorkspaceServiceAccountsDialogs.shared';
import type { WorkspaceServiceAccountsDialogsProps } from './WorkspaceServiceAccountsDialogs.types';

export function WorkspaceServiceAccountsAttachDialogs({
  attachDialogOpen,
  setAttachDialogOpen,
  attachClientId,
  setAttachClientId,
  attachError,
  busyAction,
  onAttachClient,
  onClearAttach,
  secretResult,
  secretDialogOpen,
  setSecretDialogOpen,
}: Pick<
  WorkspaceServiceAccountsDialogsProps,
  | 'attachDialogOpen'
  | 'setAttachDialogOpen'
  | 'attachClientId'
  | 'setAttachClientId'
  | 'attachError'
  | 'busyAction'
  | 'onAttachClient'
  | 'onClearAttach'
  | 'secretResult'
  | 'secretDialogOpen'
  | 'setSecretDialogOpen'
>) {
  return (
    <>
      <WorkspaceServiceAccountsAttachDialog
        attachDialogOpen={attachDialogOpen}
        setAttachDialogOpen={setAttachDialogOpen}
        attachClientId={attachClientId}
        setAttachClientId={setAttachClientId}
        attachError={attachError}
        busyAction={busyAction}
        onAttachClient={onAttachClient}
        onClearAttach={onClearAttach}
      />

      <SecretDialog
        result={secretResult}
        open={secretDialogOpen}
        onOpenChange={setSecretDialogOpen}
      />
    </>
  );
}
