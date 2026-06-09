import { WorkspaceServiceAccountsAccessDenied } from './WorkspaceServiceAccountsAccessDenied';
import { Card } from '@/components/ui/card';
import { Skeleton } from '@/components/ui/skeleton';
import { useWorkspaceServiceAccountsPage } from './useWorkspaceServiceAccountsPage';

type WorkspaceServiceAccountsPageModel = ReturnType<typeof useWorkspaceServiceAccountsPage>;

export function WorkspaceServiceAccountsSkeleton() {
  return (
    <div className="flex animate-fade-slide-up flex-col gap-6">
      <Card className="p-6">
        <Skeleton className="h-6 w-48" />
        <Skeleton className="mt-2 h-4 w-80" />
        <div className="mt-6 grid gap-3 sm:grid-cols-3">
          {Array.from({ length: 3 }).map((_, index) => (
            <Skeleton key={index} className="h-20 rounded-2xl" />
          ))}
        </div>
      </Card>
      <div className="grid gap-6 lg:grid-cols-[320px_1fr]">
        <Skeleton className="h-[28rem] rounded-3xl" />
        <Skeleton className="h-[28rem] rounded-3xl" />
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
