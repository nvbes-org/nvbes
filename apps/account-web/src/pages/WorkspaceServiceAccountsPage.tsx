import {
  WorkspaceServiceAccountsContent,
  WorkspaceServiceAccountsGuard,
} from './WorkspaceServiceAccountsPage.shared';
import { useWorkspaceServiceAccountsPage } from './useWorkspaceServiceAccountsPage';

export default function WorkspaceServiceAccountsPage() {
  const page = useWorkspaceServiceAccountsPage();

  if (page.loading || page.isLoading) {
    return <WorkspaceServiceAccountsGuard {...page} />;
  }

  if (page.workspaceId && !page.canManage) {
    return <WorkspaceServiceAccountsGuard {...page} />;
  }

  return <WorkspaceServiceAccountsContent {...page} />;
}
