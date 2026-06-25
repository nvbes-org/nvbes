import { verifiedFetch } from '@nvbes/web-runtime';
import type {
  AdminCredentials,
  AuditEvent,
  AuditEventFilters,
  AuditEvidenceExport,
  AuditEvidenceSnapshot,
  CommandCenterSnapshot,
  GlobalSearchResult,
  PendingApprovalsSnapshot,
} from './internal-admin.types';
import { authHeaders, parseJson } from './internal-admin.api.core';

export async function listAuditEvents(
  credentials: AdminCredentials,
  filters: AuditEventFilters = {},
) {
  const params = new URLSearchParams({ limit: String(filters.limit ?? 25) });
  if (filters.action?.trim()) params.set('action', filters.action.trim());
  if (filters.targetType?.trim()) params.set('target_type', filters.targetType.trim());
  if (filters.query?.trim()) params.set('q', filters.query.trim());
  const response = await verifiedFetch(
    `/workspaces/${credentials.workspaceId}/admin/audit-events?${params.toString()}`,
    { headers: authHeaders(credentials) },
  );
  return parseJson<AuditEvent[]>(response);
}

export async function exportAuditEvidence(
  credentials: AdminCredentials,
  filters: AuditEventFilters = {},
) {
  const params = new URLSearchParams();
  if (filters.action?.trim()) params.set('action', filters.action.trim());
  if (filters.targetType?.trim()) params.set('target_type', filters.targetType.trim());
  if (filters.query?.trim()) params.set('q', filters.query.trim());
  const query = params.toString();
  const response = await verifiedFetch(
    `/workspaces/${credentials.workspaceId}/admin/audit-evidence/export${query ? `?${query}` : ''}`,
    { headers: authHeaders(credentials) },
  );
  return parseJson<AuditEvidenceExport>(response);
}

export async function getCommandCenter(credentials: AdminCredentials) {
  const response = await verifiedFetch('/admin/command-center', {
    headers: authHeaders(credentials),
  });
  return parseJson<CommandCenterSnapshot>(response);
}

export async function getPendingApprovals(credentials: AdminCredentials) {
  const response = await verifiedFetch('/admin/pending-approvals', {
    headers: authHeaders(credentials),
  });
  return parseJson<PendingApprovalsSnapshot>(response);
}

export async function getAuditEvidenceCenter(credentials: AdminCredentials) {
  const response = await verifiedFetch('/admin/audit-evidence-center', {
    headers: authHeaders(credentials),
  });
  return parseJson<AuditEvidenceSnapshot>(response);
}

export async function globalSearch(credentials: AdminCredentials, query: string) {
  const params = new URLSearchParams({ q: query });
  const response = await verifiedFetch(`/admin/search?${params.toString()}`, {
    headers: authHeaders(credentials),
  });
  return parseJson<GlobalSearchResult[]>(response);
}
