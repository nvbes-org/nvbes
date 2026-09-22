import { isUsableCommitSha } from './nx-cache-manager.core.mjs';

// Never use the parent of HEAD: it can omit changes from failed CI runs.
export async function resolveScopeBase(
  { event, head, explicitBase, branch, repository, runId },
  { exists, ancestor, runs },
) {
  if (!isUsableCommitSha(head) || !exists(head)) throw new Error('Invalid CI head');
  const candidate = event.pull_request?.base?.sha ?? event.merge_group?.base_sha ?? explicitBase;
  if (candidate) {
    return isUsableCommitSha(candidate) && exists(candidate)
      ? { base: candidate, fallback: false, reason: 'explicit base' }
      : { base: head, fallback: true, reason: 'base SHA unavailable' };
  }
  if (event.pull_request) return { base: head, fallback: true, reason: 'PR base missing' };
  try {
    for await (const run of runs(branch)) {
      if (String(run.id) === String(runId) || run.head_sha === head) continue;
      if (
        run.conclusion !== 'success' ||
        run.event !== 'push' ||
        run.head_branch !== branch ||
        run.repository?.full_name !== repository ||
        run.path !== '.github/workflows/ci.yml'
      )
        continue;
      if (isUsableCommitSha(run.head_sha) && exists(run.head_sha) && ancestor(run.head_sha, head)) {
        return { base: run.head_sha, fallback: false, reason: 'last successful ancestor CI' };
      }
    }
  } catch (error) {
    return { base: head, fallback: true, reason: `CI history unavailable: ${error.message}` };
  }
  return { base: head, fallback: true, reason: 'no successful ancestor CI' };
}
