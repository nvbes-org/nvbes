import { useEffect, useState } from 'react';
import { useAccountContext } from '@/hooks/useAccountContext';

type WorkerQueueStatusView = {
  status: string;
  depth: number;
  oldest_age_seconds: number | null;
};

type WorkerQueueStatusResponse = {
  workspace_id: string;
  queue_name: string;
  snapshot_at: string;
  statuses: WorkerQueueStatusView[];
};

export default function WorkerQueueStatusPage() {
  const { me, loading: accountLoading } = useAccountContext();
  const [submittedWorkspaceId, setSubmittedWorkspaceId] = useState('');
  const [snapshot, setSnapshot] = useState<WorkerQueueStatusResponse | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');

  useEffect(() => {
    const workspaceId = submittedWorkspaceId || me?.current_workspace_id || '';

    if (!workspaceId) {
      return;
    }

    const controller = new AbortController();

    const fetchSnapshot = async () => {
      setLoading(true);
      setError('');

      try {
        const response = await fetch(`/workspaces/${workspaceId}/worker-queue/status`, {
          credentials: 'include',
          signal: controller.signal,
        });

        if (!response.ok) {
          throw new Error(`HTTP ${response.status}`);
        }

        const data = (await response.json()) as WorkerQueueStatusResponse;
        setSnapshot(data);
      } catch (fetchError) {
        if (fetchError instanceof DOMException && fetchError.name === 'AbortError') {
          return;
        }
        setError('Impossible de charger la supervision du worker.');
        setSnapshot(null);
      } finally {
        setLoading(false);
      }
    };

    fetchSnapshot();

    return () => controller.abort();
  }, [me?.current_workspace_id, submittedWorkspaceId]);

  return (
    <div className="min-h-screen bg-gradient-to-b from-slate-950 via-slate-900 to-slate-950 px-6 py-12 text-slate-100">
      <div className="mx-auto flex max-w-5xl flex-col gap-8">
        <div className="rounded-3xl border border-white/10 bg-white/5 p-8 shadow-2xl shadow-black/20 backdrop-blur">
          <p className="mb-3 text-sm uppercase tracking-[0.24em] text-cyan-300">
            Admin worker queue
          </p>
          <h1 className="text-4xl font-semibold tracking-tight">Supervision du worker Stripe</h1>
          <p className="mt-3 max-w-2xl text-sm leading-6 text-slate-300">
            Saisis un workspace pour voir l’état synthétique de la file Stripe: profondeur,
            ancienneté du plus vieux job et pression des retries.
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

        {snapshot ? (
          <div className="rounded-3xl border border-white/10 bg-slate-950/70 p-6">
            <div className="mb-4 flex items-center justify-between">
              <div>
                <h2 className="text-xl font-semibold">{snapshot.queue_name}</h2>
                <p className="text-sm text-slate-400">Workspace: {snapshot.workspace_id}</p>
              </div>
              <span className="text-sm text-slate-400">
                Snapshot {new Date(snapshot.snapshot_at).toLocaleString()}
              </span>
            </div>

            <div className="grid gap-4 md:grid-cols-2">
              {snapshot.statuses.map((status) => (
                <article
                  key={status.status}
                  className="rounded-2xl border border-white/10 bg-white/5 p-5"
                >
                  <h3 className="text-lg font-medium">{status.status}</h3>
                  <p className="mt-2 text-3xl font-semibold">{status.depth}</p>
                  <p className="mt-2 text-sm text-slate-400">
                    {status.oldest_age_seconds == null
                      ? 'Aucun job'
                      : `Plus vieux job: ${Math.round(status.oldest_age_seconds)}s`}
                  </p>
                </article>
              ))}
            </div>
          </div>
        ) : null}

        {!loading &&
        (submittedWorkspaceId || me?.current_workspace_id) &&
        snapshot == null &&
        !error ? (
          <div className="rounded-2xl border border-dashed border-white/10 p-6 text-sm text-slate-400">
            Aucun snapshot disponible pour ce workspace.
          </div>
        ) : null}
      </div>
    </div>
  );
}
