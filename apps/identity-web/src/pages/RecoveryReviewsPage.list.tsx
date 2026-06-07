import type { RecoveryReviewView } from '@/pages/RecoveryReviewsPage.api';
import { formatRecoveryReviewDateTime, recoveryReviewStatus } from './RecoveryReviewsPage.utils';

export function RecoveryReviewsList(props: {
  error: string;
  loading: boolean;
  reviews: RecoveryReviewView[];
  workspaceId: string;
}) {
  const { error, loading, reviews, workspaceId } = props;

  return (
    <>
      {error ? (
        <div className="rounded-2xl border border-red-400/20 bg-red-400/10 p-4 text-sm text-red-200">
          {error}
        </div>
      ) : null}

      {workspaceId ? (
        <div className="rounded-3xl border border-white/10 bg-slate-950/70 p-6">
          <div className="mb-4 flex items-center justify-between">
            <div>
              <h2 className="text-xl font-semibold">Demandes</h2>
              <p className="text-sm text-slate-400">Workspace: {workspaceId}</p>
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
                  <div className="text-sm text-slate-400">{recoveryReviewStatus(review)}</div>
                </div>

                <dl className="mt-4 grid gap-3 text-sm md:grid-cols-2">
                  <div>
                    <dt className="text-slate-500">Principal</dt>
                    <dd className="break-all text-slate-200">{review.principal_id}</dd>
                  </div>
                  <div>
                    <dt className="text-slate-500">Créée</dt>
                    <dd className="text-slate-200">
                      {formatRecoveryReviewDateTime(review.created_at)}
                    </dd>
                  </div>
                  <div>
                    <dt className="text-slate-500">Disponible</dt>
                    <dd className="text-slate-200">
                      {formatRecoveryReviewDateTime(review.available_at)}
                    </dd>
                  </div>
                  <div>
                    <dt className="text-slate-500">Premier approbateur</dt>
                    <dd className="text-slate-200">
                      {formatRecoveryReviewDateTime(review.approved_at)}
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
    </>
  );
}
