import { WorkspaceServiceAccountsDialogsSection } from './WorkspaceServiceAccountsPage.dialogs';
import { WorkspaceServiceAccountsMainGrid } from './WorkspaceServiceAccountsPage.grid';
import { WorkspaceServiceAccountsHeroSection } from './WorkspaceServiceAccountsPage.hero';
import type { WorkspaceServiceAccountsPageModel } from './WorkspaceServiceAccountsPage.types';

export function WorkspaceServiceAccountsContent(props: WorkspaceServiceAccountsPageModel) {
  return (
    <div className="flex flex-col gap-6 animate-fade-slide-up">
      <WorkspaceServiceAccountsHeroSection {...props} />
      <WorkspaceServiceAccountsMainGrid {...props} />
      <WorkspaceServiceAccountsDialogsSection {...props} />
    </div>
  );
}
