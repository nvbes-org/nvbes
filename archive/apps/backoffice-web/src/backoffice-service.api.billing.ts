import { verifiedFetch } from '@nvbes/web-runtime';
import type {
  AdminCredentials,
  BillingOverview,
  BillingRunbook,
  CreditNoteRequest,
  ExportType,
  GraceOverrideRequest,
  ManualCompRequest,
  MutationResult,
  ProviderEventFailure,
  ProviderMigrationRequest,
  ProviderReplayRequest,
  ProviderReplayResult,
  RefundIntentRequest,
  RunbookExecutionRequest,
  RunbookExecutionResult,
  SearchResult,
} from './backoffice-service.types';
import { authHeaders, mutationHeaders, parseJson, postJson } from './backoffice-service.api.core';

export async function searchBilling(credentials: AdminCredentials, query: string) {
  const params = new URLSearchParams({ q: query });
  const response = await verifiedFetch(
    `/workspaces/${credentials.workspaceId}/billing/admin/search?${params.toString()}`,
    { headers: authHeaders(credentials) },
  );
  return parseJson<SearchResult[]>(response);
}

export async function listBillingRunbooks(credentials: AdminCredentials) {
  const response = await verifiedFetch('/admin/billing/runbooks', {
    headers: authHeaders(credentials),
  });
  return parseJson<BillingRunbook[]>(response);
}

export function executeBillingRunbook(
  credentials: AdminCredentials,
  runbookId: string,
  body: RunbookExecutionRequest,
) {
  return postJson<RunbookExecutionRequest, RunbookExecutionResult>(
    credentials,
    `/workspaces/${credentials.workspaceId}/billing/admin/runbooks/${runbookId}/execute`,
    body,
  );
}

export async function getBillingOverview(credentials: AdminCredentials) {
  const response = await verifiedFetch(
    `/workspaces/${credentials.workspaceId}/billing/admin/overview`,
    {
      headers: authHeaders(credentials),
    },
  );
  return parseJson<BillingOverview>(response);
}

export async function listProviderEventFailures(credentials: AdminCredentials) {
  const response = await verifiedFetch(
    `/workspaces/${credentials.workspaceId}/billing/admin/provider-events/failures`,
    { headers: authHeaders(credentials) },
  );
  return parseJson<ProviderEventFailure[]>(response);
}

export function createCreditNote(credentials: AdminCredentials, body: CreditNoteRequest) {
  return postJson<CreditNoteRequest, MutationResult>(
    credentials,
    `/workspaces/${credentials.workspaceId}/billing/admin/credit-notes`,
    body,
  );
}

export function createWriteOff(credentials: AdminCredentials, body: CreditNoteRequest) {
  return postJson<CreditNoteRequest, MutationResult>(
    credentials,
    `/workspaces/${credentials.workspaceId}/billing/admin/write-offs`,
    body,
  );
}

export function createRefundIntent(credentials: AdminCredentials, body: RefundIntentRequest) {
  return postJson<RefundIntentRequest, MutationResult>(
    credentials,
    `/workspaces/${credentials.workspaceId}/billing/admin/refund-intents`,
    body,
  );
}

export function replayProviderEvent(credentials: AdminCredentials, body: ProviderReplayRequest) {
  return postJson<ProviderReplayRequest, ProviderReplayResult>(
    credentials,
    `/workspaces/${credentials.workspaceId}/billing/admin/provider-events/replay`,
    body,
  );
}

export function createProviderMigration(
  credentials: AdminCredentials,
  body: ProviderMigrationRequest,
) {
  return postJson<ProviderMigrationRequest, MutationResult>(
    credentials,
    `/workspaces/${credentials.workspaceId}/billing/admin/provider-migrations`,
    body,
  );
}

export function overrideGracePeriod(credentials: AdminCredentials, body: GraceOverrideRequest) {
  return postJson<GraceOverrideRequest, MutationResult>(
    credentials,
    `/workspaces/${credentials.workspaceId}/billing/admin/grace-overrides`,
    body,
  );
}

export function createManualCompensation(credentials: AdminCredentials, body: ManualCompRequest) {
  return postJson<ManualCompRequest, MutationResult>(
    credentials,
    `/workspaces/${credentials.workspaceId}/billing/admin/manual-compensations`,
    body,
  );
}

export async function downloadBillingExport(credentials: AdminCredentials, exportType: ExportType) {
  const response = await verifiedFetch(
    `/workspaces/${credentials.workspaceId}/billing/admin/exports/${exportType}`,
    { method: 'POST', headers: mutationHeaders(credentials) },
  );
  if (!response.ok) {
    throw new Error((await response.text()) || `Export failed with HTTP ${response.status}`);
  }
  const disposition = response.headers.get('content-disposition') ?? '';
  const filename = disposition.match(/filename="([^"]+)"/u)?.[1] ?? `${exportType}.csv`;
  return { filename, blob: await response.blob() };
}
