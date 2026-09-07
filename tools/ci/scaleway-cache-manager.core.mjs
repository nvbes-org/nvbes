export function isTrustedRef(ref) {
  return ref === 'refs/heads/main';
}

export function restorePrefixes(environment) {
  void environment;
  return ['trusted'];
}

export function writablePrefix(environment) {
  return environment.eventName === 'push' && isTrustedRef(environment.ref) ? 'trusted' : null;
}
