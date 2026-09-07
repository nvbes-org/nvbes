export function summarizeRollout(runs) {
  const successful = runs.filter((run) => run.conclusion === 'success');
  const evidence = runs
    .flatMap((run) => run.evidence ?? [])
    .filter((row) => row.baselineExecuted !== false);
  const metrics = runs.flatMap((run) => run.metrics ?? []);
  const cache = metrics.reduce(
    (sum, row) => ({
      hits: sum.hits + (row.nxCache?.hits ?? 0),
      total: sum.total + (row.nxCache?.total ?? 0),
    }),
    { hits: 0, total: 0 },
  );
  const comparisons = evidence.filter(
    (row) =>
      row.version === 1 &&
      row.event === 'pull_request' &&
      row.packages?.length < row.workspace?.length,
  );
  const divergences = evidence.filter(
    (row) => row.divergence || !row.fullSuccess || !row.scopedSuccess,
  );
  const verified = new Set(
    comparisons
      .filter((row) => row.pr && !row.divergence && row.fullSuccess && row.scopedSuccess)
      .map((row) => row.pr),
  );
  const dates = comparisons.map((row) => Date.parse(row.timestamp)).filter(Number.isFinite);
  const observationDays =
    dates.length > 1 ? (Math.max(...dates) - Math.min(...dates)) / 86_400_000 : 0;
  const durations = runs
    .flatMap((run) => run.jobs ?? [])
    .filter((job) => job.started_at && job.completed_at && job.conclusion !== 'skipped')
    .map((job) => ({
      name: job.name,
      seconds: Math.max(0, (Date.parse(job.completed_at) - Date.parse(job.started_at)) / 1000),
    }));
  return {
    runs: runs.length,
    successful: successful.length,
    nxCacheHitRate: cache.total ? cache.hits / cache.total : null,
    runnerMinutes: durations.reduce((sum, job) => sum + Math.ceil(job.seconds / 60), 0),
    jobs: durations,
    comparedPrs: verified.size,
    observationDays,
    divergences: divergences.length,
    rustEligibleForReview: verified.size >= 20 && observationDays >= 7 && divergences.length === 0,
    // Eligibility is evidence for an operator review, never automatic activation.
  };
}
