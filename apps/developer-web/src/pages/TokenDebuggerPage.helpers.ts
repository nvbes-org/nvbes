import type { DebugDeveloperToken } from '../developer.schemas';

type DecisionTone = 'success' | 'warning' | 'danger' | 'muted';

export function accessDecisionTone(result: DebugDeveloperToken | null): DecisionTone {
  if (!result) {
    return 'muted';
  }

  switch (result.access_decision) {
    case 'allowed':
      return 'success';
    case 'expired':
    case 'tenant_mismatch':
      return 'warning';
    case 'invalid':
      return 'danger';
  }
}

export function summarizeAccessDecision(result: DebugDeveloperToken | null): string {
  if (!result) {
    return 'No token inspected';
  }

  switch (result.access_decision) {
    case 'allowed':
      return 'Access allowed';
    case 'expired':
      return 'Token expired';
    case 'tenant_mismatch':
      return 'Tenant mismatch';
    case 'invalid':
      return 'Invalid token';
  }
}

export function describeAccessDecision(result: DebugDeveloperToken | null): string {
  if (!result) {
    return 'Paste an access token to inspect its decoded claims and tenant-scoped authorization result.';
  }

  switch (result.access_decision) {
    case 'allowed':
      return 'Token is active, belongs to this developer tenant, and can be accepted.';
    case 'expired':
      return 'Token decoded successfully but is outside its not-before or expiration window.';
    case 'tenant_mismatch':
      return 'Token decoded successfully but belongs to another tenant.';
    case 'invalid':
      return 'Token could not be decoded or its signature is not valid.';
  }
}

export function formatTokenTimestamp(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }

  return new Intl.DateTimeFormat('en', {
    day: 'numeric',
    hour: 'numeric',
    minute: 'numeric',
    month: 'short',
    second: 'numeric',
    year: 'numeric',
    timeZoneName: 'short',
  }).format(date);
}

export function summarizeExpiration(expiresAt: string, now: Date = new Date()): string {
  const expires = new Date(expiresAt);
  if (Number.isNaN(expires.getTime())) {
    return 'Expiration timestamp is invalid';
  }

  const seconds = Math.round((expires.getTime() - now.getTime()) / 1000);
  const absolute = formatTokenTimestamp(expiresAt);
  if (seconds === 0) {
    return `Expires now (${absolute})`;
  }

  const future = seconds > 0;
  const magnitude = Math.abs(seconds);
  const unit =
    magnitude >= 86_400
      ? ['day', Math.round(magnitude / 86_400)]
      : magnitude >= 3_600
        ? ['hour', Math.round(magnitude / 3_600)]
        : magnitude >= 60
          ? ['minute', Math.round(magnitude / 60)]
          : ['second', magnitude];
  const [label, count] = unit;
  const plural = count === 1 ? label : `${label}s`;

  return future
    ? `Expires in ${count} ${plural} (${absolute})`
    : `Expired ${count} ${plural} ago (${absolute})`;
}
