const internalAssociations = new Set(['MEMBER', 'OWNER']);

const protectedPaths = [
  '.github/workflows/',
  '.github/actions/',
  'tools/ci/configure-sccache.',
  'tools/ci/scaleway-cache-manager.',
  'tools/ci/authorize-pr-cache.',
  'infrastructure/modules/scaleway-ci-cache/',
];

export function isVerifiedGpgCommit(commit) {
  const verification = commit?.commit?.verification;
  return (
    verification?.verified === true &&
    verification.reason === 'valid' &&
    typeof verification.signature === 'string' &&
    verification.signature.includes('-----BEGIN PGP SIGNATURE-----')
  );
}

export function evaluatePullRequestCacheTrust(event, commits, files) {
  const pullRequest = event?.pull_request;
  const reasons = [];
  if (pullRequest === undefined) reasons.push('event is not a pull request');
  if (pullRequest?.head?.repo?.full_name !== event?.repository?.full_name) {
    reasons.push('pull request does not originate from the base repository');
  }
  if (!internalAssociations.has(pullRequest?.author_association)) {
    reasons.push('pull request author is not an organization member');
  }
  if (event?.sender?.login !== pullRequest?.user?.login) {
    reasons.push('workflow actor is not the pull request author');
  }
  if (!Array.isArray(commits) || commits.length === 0) {
    reasons.push('pull request has no verifiable commits');
  } else {
    if (commits.at(-1)?.sha !== pullRequest?.head?.sha) {
      reasons.push('verified commit list does not end at the pull request head');
    }
    if (!commits.every(isVerifiedGpgCommit)) {
      reasons.push('every pull request commit must have a valid GPG signature');
    }
  }
  if (files.some(({ filename = '' }) => protectedPaths.some((path) => filename.startsWith(path)))) {
    reasons.push('pull request changes the cache trust boundary');
  }
  return { trusted: reasons.length === 0, reasons };
}
