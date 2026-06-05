import { useEffect, useState } from 'react';
import { useAccountContext } from '@/hooks/useAccountContext';

type RecoveryReviewView = {
  request_id: string;
  principal_id: string;
  email: string;
  status: string;
  available_at: string;
  approved_at: string | null;
  review_available_at: string | null;
  secondary_approved_at: string | null;
  created_at: string;
  updated_at: string;
};

type RecoveryReviewsResponse = {
  workspace_id: string;
  reviews: RecoveryReviewView[];
};

export default function RecoveryReviewsPage() {
  const { me, loading: accountLoading } = useAccountContext();
  const [submittedWorkspaceId, setSubmittedWorkspaceId] = useState('');
  const [reviews, setReviews] = useState<RecoveryReviewView[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');

  useEffect(() => {
    const workspaceId = submittedWorkspaceId || me?.current_workspace_id || '';

    if (!workspaceId) {
      return;
    }

    const controller = new AbortController();

    const fetchReviews = async () => {
      setLoading(true);
      setError('');

      try {
        const response = await fetch(`/workspaces/${workspaceId}/recovery-reviews`, {
          credentials: 'include',
          signal: controller.signal,
        });

        if (!response.ok) {
          throw new Error(`HTTP ${response.status}`);
        }

        const data = (await response.json()) as RecoveryReviewsResponse;
        setReviews(data.reviews || []);
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
  }, [me?.current_workspace_id, submittedWorkspaceId]);

  return (
    <div className="min-h-screen bg-gradient-to-b from-slate-950 via-slate-900 to-slate-950 px-6 py-12 text-slate-100">
      <div className="mx-auto flex max-w-5xl flex-col gap-8">
        <div className="rounded-3xl border border-white/10 bg-white/5 p-8 shadow-2xl shadow-black/20 backdrop-blur">
          <p className="mb-3 text-sm uppercase tracking-[0.24em] text-cyan-300">
            Admin recovery queue
          </p>
          <h1 className="text-4xl font-semibold tracking-tight">
            Revue des récupérations entreprise
          </h1>
          <p className="mt-3 max-w-2xl text-sm leading-6 text-slate-300">
            Saisis un workspace pour voir les demandes `first_approved` en attente de revue finale.
            La file affiche le délai, le statut et la double approbation.
          </p>

          <div className="mt-6 flex flex-col gap-3 md:flex-row">
            <input
              value={submittedWorkspaceId || me?.current_workspace_id || ''}
              onChange={(event) => setSubmittedWorkspaceId(event.target.value)}
              placeholder={me?.current_workspace_id ?? 'Workspace ID'}
              className="h-12 flex-1 rounded-xl border border-white/10 bg-slate-950/60 px-4 text-sm text-slate-100 outline-none ring-0 placeholder:text-slate-500 focus:border-cyan-400"
            />
            <button
              type="button"
              onClick={() => setSubmittedWorkspaceId((me?.current_workspace_id ?? '').trim())}
              disabled={!me?.current_workspace_id || loading || accountLoading}
              className="h-12 rounded-xl bg-cyan-500 px-6 text-slate-950 transition hover:bg-cyan-400 disabled:cursor-not-allowed disabled:opacity-60"
            >
              Charger
            </button>
          </div>
        </div>

        {error ? (
          <div className="rounded-2xl border border-red-400/20 bg-red-400/10 p-4 text-sm text-red-200">
            {error}
          </div>
        ) : null}

        {submittedWorkspaceId || me?.current_workspace_id ? (
          <div className="rounded-3xl border border-white/10 bg-slate-950/70 p-6">
            <div className="mb-4 flex items-center justify-between">
              <div>
                <h2 className="text-xl font-semibold">Demandes</h2>
                <p className="text-sm text-slate-400">
                  Workspace: {submittedWorkspaceId || me?.current_workspace_id}
                </p>
              </div>
              {loading ? (
                <span className="text-sm text-slate-400">Chargement...</span>
              ) : (
                <span className="text-sm text-slate-400">{reviews.length} entrée(s)</span>
              )}
            </div>

            <div className="grid gap-4">
              {reviews.map((review) => (
                <article
                  key={review.request_id}
                  className="rounded-2xl border border-white/10 bg-white/5 p-5"
                >
                  <div className="flex flex-col gap-2 md:flex-row md:items-start md:justify-between">
                    <div>
                      <h3 className="text-lg font-medium">{review.email}</h3>
                      <p className="text-sm text-slate-400">
                        Request {review.request_id} · {review.status}
                      </p>
                    </div>
                    <div className="text-sm text-slate-400">
                      {review.secondary_approved_at
                        ? 'Double approbation obtenue'
                        : review.review_available_at
                          ? `Revue disponible à ${new Date(review.review_available_at).toLocaleString()}`
                          : 'En attente de première approbation'}
                    </div>
                  </div>

                  <dl className="mt-4 grid gap-3 text-sm md:grid-cols-2">
                    <div>
                      <dt className="text-slate-500">Principal</dt>
                      <dd className="break-all text-slate-200">{review.principal_id}</dd>
                    </div>
                    <div>
                      <dt className="text-slate-500">Créée</dt>
                      <dd className="text-slate-200">
                        {new Date(review.created_at).toLocaleString()}
                      </dd>
                    </div>
                    <div>
                      <dt className="text-slate-500">Disponible</dt>
                      <dd className="text-slate-200">
                        {new Date(review.available_at).toLocaleString()}
                      </dd>
                    </div>
                    <div>
                      <dt className="text-slate-500">Premier approbateur</dt>
                      <dd className="text-slate-200">
                        {review.approved_at ? new Date(review.approved_at).toLocaleString() : '—'}
                      </dd>
                    </div>
                  </dl>
                </article>
              ))}

              {!loading && reviews.length === 0 ? (
                <p className="rounded-2xl border border-dashed border-white/10 p-6 text-sm text-slate-400">
                  Aucune demande de recovery en revue pour ce workspace.
                </p>
              ) : null}
            </div>
          </div>
        ) : null}
      </div>
    </div>
  );
}
