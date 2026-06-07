export function RecoveryReviewsHeader(props: {
  accountLoading: boolean;
  currentWorkspaceId: string;
  loading: boolean;
  onChangeWorkspaceId: (value: string) => void;
  onLoadCurrentWorkspace: () => void;
  submittedWorkspaceId: string;
}) {
  const {
    accountLoading,
    currentWorkspaceId,
    loading,
    onChangeWorkspaceId,
    onLoadCurrentWorkspace,
    submittedWorkspaceId,
  } = props;

  return (
    <div className="rounded-3xl border border-white/10 bg-white/5 p-8 shadow-2xl shadow-black/20 backdrop-blur">
      <p className="mb-3 text-sm uppercase tracking-[0.24em] text-cyan-300">Admin recovery queue</p>
      <h1 className="text-4xl font-semibold tracking-tight">Revue des récupérations entreprise</h1>
      <p className="mt-3 max-w-2xl text-sm leading-6 text-slate-300">
        Saisis un workspace pour voir les demandes `first_approved` en attente de revue finale. La
        file affiche le délai, le statut et la double approbation.
      </p>

      <div className="mt-6 flex flex-col gap-3 md:flex-row">
        <input
          value={submittedWorkspaceId || currentWorkspaceId}
          onChange={(event) => onChangeWorkspaceId(event.target.value)}
          placeholder={currentWorkspaceId || 'Workspace ID'}
          className="h-12 flex-1 rounded-xl border border-white/10 bg-slate-950/60 px-4 text-sm text-slate-100 outline-none ring-0 placeholder:text-slate-500 focus:border-cyan-400"
        />
        <button
          type="button"
          onClick={onLoadCurrentWorkspace}
          disabled={!currentWorkspaceId || loading || accountLoading}
          className="h-12 rounded-xl bg-cyan-500 px-6 text-slate-950 transition hover:bg-cyan-400 disabled:cursor-not-allowed disabled:opacity-60"
        >
          Charger
        </button>
      </div>
    </div>
  );
}
