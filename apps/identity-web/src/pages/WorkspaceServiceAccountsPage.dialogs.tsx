import { WorkspaceServiceAccountsDialogs } from './WorkspaceServiceAccountsDialogs';
import { getWorkspaceServiceAccountsDialogsProps } from './WorkspaceServiceAccountsPage.dialogs.props';
import type { WorkspaceServiceAccountsDialogsSectionProps } from './WorkspaceServiceAccountsPage.dialogs.types';

export function WorkspaceServiceAccountsDialogsSection(
  props: WorkspaceServiceAccountsDialogsSectionProps,
) {
  return <WorkspaceServiceAccountsDialogs {...getWorkspaceServiceAccountsDialogsProps(props)} />;
}
