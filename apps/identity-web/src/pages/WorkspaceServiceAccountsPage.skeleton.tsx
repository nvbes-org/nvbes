import { WorkspaceServiceAccountsAccessDenied } from './WorkspaceServiceAccountsAccessDenied';
import { useWorkspaceServiceAccountsPage } from './useWorkspaceServiceAccountsPage';

type WorkspaceServiceAccountsPageModel = ReturnType<typeof useWorkspaceServiceAccountsPage>;

export function WorkspaceServiceAccountsSkeleton() {
  return (
    <div className="flex animate-fade-slide-up flex-col gap-6">
      <div className="rounded-3xl border border-border/70 bg-card p-6">
        <div className="h-6 w-48 rounded bg-muted" />
        <div className="mt-2 h-4 w-80 rounded bg-muted" />
        <div className="mt-6 grid gap-3 sm:grid-cols-3">
          {Array.from({ length: 3 }).map((_, index) => (
            <div key={index} className="h-20 rounded-2xl bg-muted" />
          ))}
        </div>
      </div>
      <div className="grid gap-6 lg:grid-cols-[320px_1fr]">
        <div className="h-[28rem] rounded-3xl bg-muted" />
        <div className="h-[28rem] rounded-3xl bg-muted" />
      </div>
    </div>
  );
}

export function WorkspaceServiceAccountsGuard({
  loading,
  workspaceId,
  canManage,
  workspace,
  isLoading,
}: Pick<
  WorkspaceServiceAccountsPageModel,
  'loading' | 'workspaceId' | 'canManage' | 'workspace' | 'isLoading'
>) {
  if (loading || isLoading) {
    return <WorkspaceServiceAccountsSkeleton />;
  }

  if (workspaceId && !canManage) {
    return <WorkspaceServiceAccountsAccessDenied workspace={workspace} workspaceId={workspaceId} />;
  }

  return null;
}
