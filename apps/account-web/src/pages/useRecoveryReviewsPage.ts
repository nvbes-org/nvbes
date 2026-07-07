import { useEffect, useState } from 'react';
import { useAccountContext } from '@/hooks/useAccountContext';
import { listRecoveryReviews, type RecoveryReviewView } from '@/pages/RecoveryReviewsPage.api';

export function useRecoveryReviewsPage() {
  const { me, loading: accountLoading } = useAccountContext();
  const [submittedWorkspaceId, setSubmittedWorkspaceId] = useState('');
  const [reviews, setReviews] = useState<RecoveryReviewView[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');

  const workspaceId = submittedWorkspaceId || me?.current_workspace_id || '';

  useEffect(() => {
    if (!workspaceId) {
      return;
    }

    const controller = new AbortController();

    const fetchReviews = async () => {
      setLoading(true);
      setError('');

      try {
        setReviews(await listRecoveryReviews(workspaceId, controller.signal));
      } catch (fetchError) {
        if (fetchError instanceof DOMException && fetchError.name === 'AbortError') {
          return;
        }
        setError('Impossible de charger la file de revue recovery.');
        setReviews([]);
      } finally {
        setLoading(false);
      }
    };

    void fetchReviews();

    return () => controller.abort();
  }, [workspaceId]);

  return {
    accountLoading,
    error,
    loading,
    reviews,
    submittedWorkspaceId,
    workspaceId,
    currentWorkspaceId: me?.current_workspace_id ?? '',
    setSubmittedWorkspaceId,
  };
}
