import { join } from 'node:path';

const trustedRefs = new Set(['refs/heads/main', 'refs/heads/dev', 'refs/heads/staging']);

export function isTrustedPush(eventName, ref) {
  return eventName === 'push' && (trustedRefs.has(ref) || ref.startsWith('refs/heads/release/'));
}

export function isUsableCommitSha(value) {
  return /^[0-9a-f]{40}$/u.test(value) && !/^0{40}$/u.test(value);
}

export function nxMajor(version) {
  const match = version.match(/(\d+)\./u);
  if (match === null) throw new Error(`Unsupported Nx version: ${version}`);
  return match[1];
}

function safeSegment(value) {
  const segment = value.toLowerCase().replaceAll(/[^a-z0-9_-]/gu, '-');
  if (segment.length === 0) throw new Error('Cache path segment cannot be empty');
  return segment;
}

export function trustedCacheDirectory({ runnerToolCache, runnerOs, runnerArch, nxVersion }) {
  return join(
    runnerToolCache,
    'nvbes',
    'nx-cache',
    'v1',
    `${safeSegment(runnerOs)}-${safeSegment(runnerArch)}`,
    `nx-${nxMajor(nxVersion)}`,
    'trusted',
  );
}

export function selectTypeScriptProjects(affectedProjects, graphNodes) {
  return affectedProjects.filter((project) => {
    const node = graphNodes[project];
    const targets = node?.data?.targets;
    return (
      node?.data?.root?.startsWith('libs/ts/') === true &&
      targets !== undefined &&
      ['format:check', 'lint', 'typecheck'].every((target) => Object.hasOwn(targets, target))
    );
  });
}
