import { Alert, AlertDescription } from '@/components/ui/alert';
import { Badge } from '@/components/ui/badge';
import { Card } from '@/components/ui/card';
import { Empty, EmptyDescription, EmptyHeader, EmptyTitle } from '@/components/ui/empty';
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
        <Alert variant="destructive">
          <AlertDescription>{error}</AlertDescription>
        </Alert>
      ) : null}

      {workspaceId ? (
        <Card className="border-white/10 bg-slate-950/70 p-6 text-slate-100">
          <div className="mb-4 flex items-center justify-between">
            <div>
              <h2 className="text-xl font-semibold">Demandes</h2>
              <p className="text-sm text-slate-400">Workspace: {workspaceId}</p>
            </div>
            {loading ? (
              <Badge variant="secondary">Chargement...</Badge>
            ) : (
              <Badge variant="secondary">{reviews.length} entrée(s)</Badge>
            )}
          </div>

          <div className="grid gap-4">
            {reviews.map((review) => (
              <Card
                key={review.request_id}
                className="border-white/10 bg-white/5 p-5 text-slate-100"
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
              </Card>
            ))}

            {!loading && reviews.length === 0 ? (
              <Empty className="border border-white/10 text-slate-400">
                <EmptyHeader>
                  <EmptyTitle>Aucune demande</EmptyTitle>
                  <EmptyDescription>
                    Aucune demande de recovery en revue pour ce workspace.
                  </EmptyDescription>
                </EmptyHeader>
              </Empty>
            ) : null}
          </div>
        </Card>
      ) : null}
    </>
  );
}
