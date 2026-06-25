import type { AuditEvent } from './internal-admin.types';
import type { AuditSortMode } from './internal-admin.audit-view-state';

const sensitiveTokens = ['delete', 'revoke', 'suspend', 'refund', 'break', 'hold', 'block'];

export function sortAuditEvents(events: AuditEvent[], sort: AuditSortMode): AuditEvent[] {
  return [...events].sort((left, right) => compareAuditEvents(left, right, sort));
}

function compareAuditEvents(left: AuditEvent, right: AuditEvent, sort: AuditSortMode): number {
  if (sort === 'hash_anomalies') {
    return byBoolean(isHashAnomaly(right), isHashAnomaly(left)) || byNewest(left, right);
  }
  if (sort === 'oldest') return byOldest(left, right);
  if (sort === 'sensitive_first') {
    return byBoolean(isSensitive(right), isSensitive(left)) || byNewest(left, right);
  }
  return byNewest(left, right);
}

function byNewest(left: AuditEvent, right: AuditEvent): number {
  return Date.parse(right.created_at) - Date.parse(left.created_at);
}

function byOldest(left: AuditEvent, right: AuditEvent): number {
  return Date.parse(left.created_at) - Date.parse(right.created_at);
}

function byBoolean(left: boolean, right: boolean): number {
  return Number(left) - Number(right);
}

function isHashAnomaly(event: AuditEvent): boolean {
  return event.hash_chain_status === 'hash_anomaly';
}

function isSensitive(event: AuditEvent): boolean {
  return sensitiveTokens.some((token) => event.action.toLowerCase().includes(token));
}
