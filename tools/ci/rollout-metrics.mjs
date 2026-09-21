export function scopeMetric(plan, startedAt) {
  const lanes = Object.entries(plan.candidate)
    .filter(([name, required]) => name !== 'contracts' && required)
    .map(([name]) => name)
    .sort((a, b) => a.localeCompare(b));
  const category = plan.fallbackFull
    ? 'fallback-full'
    : plan.docsOnly
      ? 'docs'
      : plan.ciOnly
        ? 'ci'
        : lanes.join('+') || 'contracts';
  return {
    version: 1,
    head: plan.head,
    base: plan.base,
    category,
    shadow: plan.shadow,
    fallbackFull: plan.fallbackFull,
    rustMode: plan.rustMode,
    candidate: plan.candidate,
    planningMs: Date.now() - startedAt,
  };
}

export function runMeasurements(runs) {
  return runs.map((run) => {
    const jobs = (run.jobs ?? []).filter(
      (job) => job.conclusion !== 'skipped' && job.started_at && job.completed_at,
    );
    const starts = jobs.map((job) => Date.parse(job.started_at)).filter(Number.isFinite);
    const ends = jobs.map((job) => Date.parse(job.completed_at)).filter(Number.isFinite);
    const scope = run.scopes?.at(-1);
    return {
      id: run.id,
      url: run.html_url,
      conclusion: run.conclusion,
      event: run.event,
      category: scope?.category ?? 'unclassified',
      shadow: scope?.shadow ?? null,
      planningMs: scope?.planningMs ?? null,
      elapsedSeconds:
        starts.length && ends.length ? (Math.max(...ends) - Math.min(...starts)) / 1000 : null,
      runnerMinutes: jobs.reduce((total, job) => {
        const seconds = (Date.parse(job.completed_at) - Date.parse(job.started_at)) / 1000;
        return total + (Number.isFinite(seconds) ? Math.ceil(Math.max(0, seconds) / 60) : 0);
      }, 0),
    };
  });
}
