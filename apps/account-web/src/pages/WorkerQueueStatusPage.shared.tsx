import { Alert, AlertDescription } from '@/components/ui/alert';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card } from '@/components/ui/card';
import { Empty, EmptyDescription, EmptyHeader, EmptyTitle } from '@/components/ui/empty';
import { Input } from '@/components/ui/input';
import type { useWorkerQueueStatusPage } from './useWorkerQueueStatusPage';

type WorkerQueueStatusPageContentProps = ReturnType<typeof useWorkerQueueStatusPage>;

export function WorkerQueueStatusPageContent({
  me,
  accountLoading,
  submittedWorkspaceId,
  setSubmittedWorkspaceId,
  snapshot,
  loading,
  error,
}: WorkerQueueStatusPageContentProps) {
  return (
    <div className="min-h-screen bg-gradient-to-b from-slate-950 via-slate-900 to-slate-950 px-6 py-12 text-slate-100">
      <div className="mx-auto flex max-w-5xl flex-col gap-8">
        <Card className="border-white/10 bg-white/5 p-8 text-slate-100 shadow-2xl shadow-black/20 backdrop-blur">
          <p className="mb-3 text-sm uppercase tracking-[0.24em] text-cyan-300">
            Admin worker queue
          </p>
          <h1 className="text-4xl font-semibold tracking-tight">Supervision du worker Identity</h1>
          <p className="mt-3 max-w-2xl text-sm leading-6 text-slate-300">
            Saisis un workspace pour voir l’état synthétique de la file email: profondeur,
            ancienneté du plus vieux job et pression des retries.
          </p>

          <div className="mt-6 flex flex-col gap-3 md:flex-row">
            <Input
              value={submittedWorkspaceId || me?.current_workspace_id || ''}
              onChange={(event) => setSubmittedWorkspaceId(event.target.value)}
              placeholder={me?.current_workspace_id ?? 'Workspace ID'}
              className="h-12 flex-1 border-white/10 bg-slate-950/60 text-slate-100 placeholder:text-slate-500"
            />
            <Button
              type="button"
              onClick={() => setSubmittedWorkspaceId((me?.current_workspace_id ?? '').trim())}
              disabled={!me?.current_workspace_id || loading || accountLoading}
              className="h-12 px-6"
            >
              Charger
            </Button>
          </div>
        </Card>

        {error ? (
          <Alert variant="destructive">
            <AlertDescription>{error}</AlertDescription>
          </Alert>
        ) : null}

        {snapshot ? (
          <Card className="border-white/10 bg-slate-950/70 p-6 text-slate-100">
            <div className="mb-4 flex items-center justify-between">
              <div>
                <h2 className="text-xl font-semibold">{snapshot.queue_name}</h2>
                <p className="text-sm text-slate-400">Workspace: {snapshot.workspace_id}</p>
              </div>
              <Badge variant="secondary">
                Snapshot {new Date(snapshot.snapshot_at).toLocaleString()}
              </Badge>
            </div>

            <div className="grid gap-4 md:grid-cols-2">
              {snapshot.statuses.map((status) => (
                <Card key={status.status} className="border-white/10 bg-white/5 p-5 text-slate-100">
                  <h3 className="text-lg font-medium">{status.status}</h3>
                  <p className="mt-2 text-3xl font-semibold">{status.depth}</p>
                  <p className="mt-2 text-sm text-slate-400">
                    {status.oldest_age_seconds == null
                      ? 'Aucun job'
                      : `Plus vieux job: ${Math.round(status.oldest_age_seconds)}s`}
                  </p>
                </Card>
              ))}
            </div>
          </Card>
        ) : null}

        {!loading &&
        (submittedWorkspaceId || me?.current_workspace_id) &&
        snapshot == null &&
        !error ? (
          <Empty className="border border-white/10 text-slate-400">
            <EmptyHeader>
              <EmptyTitle>Aucun snapshot</EmptyTitle>
              <EmptyDescription>Aucun snapshot disponible pour ce workspace.</EmptyDescription>
            </EmptyHeader>
          </Empty>
        ) : null}
      </div>
    </div>
  );
}
