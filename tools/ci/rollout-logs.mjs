export function collectRunLogs(run, jobs, repository, gh) {
  const args = ['run', 'view', String(run.id), '--repo', repository];
  try {
    return gh(...args, '--log');
  } catch (error) {
    const missing = String(error.stderr ?? '').match(/log not found: (\d+)/u)?.[1];
    const job = jobs.find((candidate) => String(candidate.id) === missing);
    if (
      !job ||
      !Array.isArray(job.steps) ||
      job.steps.length !== 0 ||
      !['cancelled', 'skipped'].includes(job.conclusion)
    )
      throw error;
    // GitHub does not produce logs for jobs cancelled before a runner starts.
    // Read every executed job individually; missing execution logs still fail.
    return jobs
      .filter((candidate) => candidate.steps?.length > 0)
      .map((candidate) => gh(...args, '--job', String(candidate.id), '--log'))
      .join('\n');
  }
}
