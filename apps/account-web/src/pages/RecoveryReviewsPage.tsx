import { RecoveryReviewsHeader, RecoveryReviewsList } from '@/pages/RecoveryReviewsPage.shared';
import { useRecoveryReviewsPage } from '@/pages/useRecoveryReviewsPage';

export default function RecoveryReviewsPage() {
  const {
    accountLoading,
    currentWorkspaceId,
    error,
    loading,
    reviews,
    setSubmittedWorkspaceId,
    submittedWorkspaceId,
    workspaceId,
  } = useRecoveryReviewsPage();

  return (
    <div className="min-h-screen bg-gradient-to-b from-slate-950 via-slate-900 to-slate-950 px-6 py-12 text-slate-100">
      <div className="mx-auto flex max-w-5xl flex-col gap-8">
        <RecoveryReviewsHeader
          accountLoading={accountLoading}
          currentWorkspaceId={currentWorkspaceId}
          loading={loading}
          onChangeWorkspaceId={setSubmittedWorkspaceId}
          onLoadCurrentWorkspace={() => setSubmittedWorkspaceId(currentWorkspaceId.trim())}
          submittedWorkspaceId={submittedWorkspaceId}
        />
        <RecoveryReviewsList
          error={error}
          loading={loading}
          reviews={reviews}
          workspaceId={workspaceId}
        />
      </div>
    </div>
  );
}
