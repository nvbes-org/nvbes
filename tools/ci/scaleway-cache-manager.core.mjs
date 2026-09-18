export function isTrustedRef(ref) {
  return ref === 'refs/heads/main';
}

export function restorePrefixes(environment = {}) {
  const prefixes = [];
  if (environment.isTrustedPr && environment.pullRequestNumber) {
    prefixes.push(`branches/pr-${environment.pullRequestNumber}`);
  }
  prefixes.push('trusted');
  return prefixes;
}

export function writablePrefix(environment = {}) {
  if (environment.eventName === 'push' && isTrustedRef(environment.ref)) {
    return 'trusted';
  }
  if (
    environment.eventName === 'pull_request' &&
    environment.isTrustedPr &&
    environment.pullRequestNumber
  ) {
    return `branches/pr-${environment.pullRequestNumber}`;
  }
  return null;
}
