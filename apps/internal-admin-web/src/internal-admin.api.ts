import { verifiedFetch } from '@nvbes/web-runtime';
import type {
  AccessCenterSnapshot,
  AccessActionRequest,
  AccessActionResult,
  AdminCredentials,
  AuditEventFilters,
  AuditEvidenceSnapshot,
  AuditEvent,
  BillingPlatformSnapshot,
  BillingOverview,
  BillingRunbook,
  CommandCenterSnapshot,
  CommunicationsActionRequest,
  CommunicationsActionResult,
  CommunicationsCenterSnapshot,
  ComplianceActionRequest,
  ComplianceActionResult,
  ComplianceCenterSnapshot,
  CreditNoteRequest,
  CustomerCenterSnapshot,
  DeveloperActionRequest,
  DeveloperActionResult,
  DeveloperCenterSnapshot,
  EntitlementActionResult,
  EntitlementFeatureActionRequest,
  EntitlementPublishRequest,
  EntitlementQuotaOverrideRequest,
  EntitlementsSnapshot,
  EmailSuppressionRequest,
  ExportType,
  GraceOverrideRequest,
  GlobalSearchResult,
  GovernanceActionRequest,
  GovernanceActionResult,
  IdentityGovernanceSnapshot,
  ManualCompRequest,
  MutationResult,
  OperationsCenterSnapshot,
  OperationsActionRequest,
  OperationsActionResult,
  ProviderMigrationRequest,
  ProviderEventFailure,
  ProviderReplayRequest,
  ProviderReplayResult,
  RegionActionResult,
  RegionCenterSnapshot,
  RegionExceptionRequest,
  RegionFlagRequest,
  RefundIntentRequest,
  RevenueActionRequest,
  RevenueActionResult,
  RevenueCenterSnapshot,
  RiskActionRequest,
  RiskActionResult,
  RiskDecisionSnapshot,
  RunbookExecutionRequest,
  RunbookExecutionResult,
  SearchResult,
  SecurityCenterSnapshot,
  SecurityActionRequest,
  SecurityActionResult,
  SuppressionReviewRequest,
  TenantDetail,
  TenantLifecycleRequest,
  TenantLifecycleResult,
  FreezeMeterRequest,
  UsageCenterSnapshot,
  UsageActionResult,
  UsageCorrectionRequest,
  UserDetail,
  UserLifecycleRequest,
  UserLifecycleResult,
  WorkspaceDetail,
  WorkspaceLifecycleRequest,
  WorkspaceLifecycleResult,
  ReplayUsageRollupRequest,
} from './internal-admin.types';

const jsonHeaders = { 'content-type': 'application/json' };

function authHeaders(credentials: AdminCredentials): Record<string, string> {
  return {
    'x-nvbes-internal-token': credentials.internalToken,
    'x-nvbes-actor-principal-id': credentials.actorPrincipalId,
    'x-nvbes-backoffice-role': credentials.backofficeRole,
  };
}

function mutationHeaders(credentials: AdminCredentials): Record<string, string> {
  return {
    ...authHeaders(credentials),
    'Idempotency-Key': crypto.randomUUID(),
  };
}

async function parseJson<T>(response: Response): Promise<T> {
  if (!response.ok) {
    const text = await response.text();
    throw new Error(text || `Back-office request failed with HTTP ${response.status}`);
  }
  return (await response.json()) as T;
}

async function postJson<TBody, TResult>(
  credentials: AdminCredentials,
  path: string,
  body: TBody,
): Promise<TResult> {
  const response = await verifiedFetch(path, {
    method: 'POST',
    headers: { ...jsonHeaders, ...mutationHeaders(credentials) },
    body: JSON.stringify(body),
  });
  return parseJson<TResult>(response);
}

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

export async function getCommandCenter(credentials: AdminCredentials) {
  const response = await verifiedFetch('/admin/command-center', {
    headers: authHeaders(credentials),
  });
  return parseJson<CommandCenterSnapshot>(response);
}

export async function getAccessCenter(credentials: AdminCredentials) {
  const response = await verifiedFetch('/admin/access-center', { headers: authHeaders(credentials) });
  return parseJson<AccessCenterSnapshot>(response);
}

export function suspendWorkspaceMembership(
  credentials: AdminCredentials,
  workspaceId: string,
  principalId: string,
  body: AccessActionRequest,
) {
  return postJson<AccessActionRequest, AccessActionResult>(
    credentials,
    `/admin/access-center/workspace-memberships/${workspaceId}/${principalId}/suspend`,
    body,
  );
}

export async function getAuditEvidenceCenter(credentials: AdminCredentials) {
  const response = await verifiedFetch('/admin/audit-evidence-center', {
    headers: authHeaders(credentials),
  });
  return parseJson<AuditEvidenceSnapshot>(response);
}

export async function getSecurityCenter(credentials: AdminCredentials) {
  const response = await verifiedFetch('/admin/security-center', { headers: authHeaders(credentials) });
  return parseJson<SecurityCenterSnapshot>(response);
}

export function revokeMfaFactor(
  credentials: AdminCredentials,
  factorId: string,
  body: SecurityActionRequest,
) {
  return postJson<SecurityActionRequest, SecurityActionResult>(
    credentials,
    `/admin/security-center/mfa-factors/${factorId}/revoke`,
    body,
  );
}

export function revokeOauthConsent(
  credentials: AdminCredentials,
  consentId: string,
  body: SecurityActionRequest,
) {
  return postJson<SecurityActionRequest, SecurityActionResult>(
    credentials,
    `/admin/security-center/oauth-consents/${consentId}/revoke`,
    body,
  );
}

export async function getComplianceCenter(credentials: AdminCredentials) {
  const response = await verifiedFetch('/admin/compliance-center', { headers: authHeaders(credentials) });
  return parseJson<ComplianceCenterSnapshot>(response);
}

export function revokeComplianceConsent(
  credentials: AdminCredentials,
  consentId: string,
  body: ComplianceActionRequest,
) {
  return postJson<ComplianceActionRequest, ComplianceActionResult>(
    credentials,
    `/workspaces/${credentials.workspaceId}/admin/compliance/consents/${consentId}/revoke`,
    body,
  );
}

export function requestComplianceErasure(
  credentials: AdminCredentials,
  principalId: string,
  body: ComplianceActionRequest,
) {
  return postJson<ComplianceActionRequest, ComplianceActionResult>(
    credentials,
    `/workspaces/${credentials.workspaceId}/admin/compliance/principals/${principalId}/erasure-request`,
    body,
  );
}

export function reviewComplianceSuppression(
  credentials: AdminCredentials,
  body: SuppressionReviewRequest,
) {
  return postJson<SuppressionReviewRequest, ComplianceActionResult>(
    credentials,
    `/workspaces/${credentials.workspaceId}/admin/compliance/suppressions/review`,
    body,
  );
}

export async function getCommunicationsCenter(credentials: AdminCredentials) {
  const response = await verifiedFetch('/admin/communications-center', {
    headers: authHeaders(credentials),
  });
  return parseJson<CommunicationsCenterSnapshot>(response);
}

export function replayEmailMessage(
  credentials: AdminCredentials,
  messageId: string,
  body: CommunicationsActionRequest,
) {
  return postJson<CommunicationsActionRequest, CommunicationsActionResult>(
    credentials,
    `/workspaces/${credentials.workspaceId}/admin/communications/emails/${messageId}/replay`,
    body,
  );
}

export function replayEmailWebhook(
  credentials: AdminCredentials,
  eventId: string,
  body: CommunicationsActionRequest,
) {
  return postJson<CommunicationsActionRequest, CommunicationsActionResult>(
    credentials,
    `/workspaces/${credentials.workspaceId}/admin/communications/webhooks/${eventId}/replay`,
    body,
  );
}

export function suppressEmail(credentials: AdminCredentials, body: EmailSuppressionRequest) {
  return postJson<EmailSuppressionRequest, CommunicationsActionResult>(
    credentials,
    `/workspaces/${credentials.workspaceId}/admin/communications/suppressions`,
    body,
  );
}

export function unsuppressEmail(credentials: AdminCredentials, body: EmailSuppressionRequest) {
  return postJson<EmailSuppressionRequest, CommunicationsActionResult>(
    credentials,
    `/workspaces/${credentials.workspaceId}/admin/communications/suppressions/remove`,
    body,
  );
}

export async function getRegionCenter(credentials: AdminCredentials) {
  const response = await verifiedFetch('/admin/region-center', { headers: authHeaders(credentials) });
  return parseJson<RegionCenterSnapshot>(response);
}

export function flagRegionResidency(
  credentials: AdminCredentials,
  targetWorkspaceId: string,
  body: RegionFlagRequest,
) {
  return postJson<RegionFlagRequest, RegionActionResult>(
    credentials,
    `/workspaces/${credentials.workspaceId}/admin/region/workspaces/${targetWorkspaceId}/residency-flag`,
    body,
  );
}

export function recordRegionException(
  credentials: AdminCredentials,
  targetWorkspaceId: string,
  body: RegionExceptionRequest,
) {
  return postJson<RegionExceptionRequest, RegionActionResult>(
    credentials,
    `/workspaces/${credentials.workspaceId}/admin/region/workspaces/${targetWorkspaceId}/exceptions`,
    body,
  );
}

export async function getRevenueCenter(credentials: AdminCredentials) {
  const response = await verifiedFetch('/admin/revenue-center', { headers: authHeaders(credentials) });
  return parseJson<RevenueCenterSnapshot>(response);
}

export function closeDunningCase(
  credentials: AdminCredentials,
  caseId: string,
  body: RevenueActionRequest,
) {
  return postJson<RevenueActionRequest, RevenueActionResult>(
    credentials,
    `/workspaces/${credentials.workspaceId}/admin/revenue/dunning-cases/${caseId}/close`,
    body,
  );
}

export function reopenDunningCase(
  credentials: AdminCredentials,
  caseId: string,
  body: RevenueActionRequest,
) {
  return postJson<RevenueActionRequest, RevenueActionResult>(
    credentials,
    `/workspaces/${credentials.workspaceId}/admin/revenue/dunning-cases/${caseId}/reopen`,
    body,
  );
}

export function holdRevenueInvoice(
  credentials: AdminCredentials,
  invoiceId: string,
  body: RevenueActionRequest,
) {
  return postJson<RevenueActionRequest, RevenueActionResult>(
    credentials,
    `/workspaces/${credentials.workspaceId}/admin/revenue/invoices/${invoiceId}/hold`,
    body,
  );
}

export function releaseRevenueInvoice(
  credentials: AdminCredentials,
  invoiceId: string,
  body: RevenueActionRequest,
) {
  return postJson<RevenueActionRequest, RevenueActionResult>(
    credentials,
    `/workspaces/${credentials.workspaceId}/admin/revenue/invoices/${invoiceId}/release`,
    body,
  );
}

export function reviewRevenueDispute(
  credentials: AdminCredentials,
  disputeId: string,
  body: RevenueActionRequest,
) {
  return postJson<RevenueActionRequest, RevenueActionResult>(
    credentials,
    `/workspaces/${credentials.workspaceId}/admin/revenue/disputes/${disputeId}/review`,
    body,
  );
}

export function resolveRevenueDispute(
  credentials: AdminCredentials,
  disputeId: string,
  body: RevenueActionRequest,
) {
  return postJson<RevenueActionRequest, RevenueActionResult>(
    credentials,
    `/workspaces/${credentials.workspaceId}/admin/revenue/disputes/${disputeId}/resolve`,
    body,
  );
}

export async function getRiskDecisionCenter(credentials: AdminCredentials) {
  const response = await verifiedFetch('/admin/risk-decision-center', {
    headers: authHeaders(credentials),
  });
  return parseJson<RiskDecisionSnapshot>(response);
}

export function approveRiskPolicy(
  credentials: AdminCredentials,
  policyId: string,
  body: RiskActionRequest,
) {
  return postJson<RiskActionRequest, RiskActionResult>(
    credentials,
    `/workspaces/${credentials.workspaceId}/admin/risk/policies/${policyId}/approve`,
    body,
  );
}

export function blockRiskPolicy(
  credentials: AdminCredentials,
  policyId: string,
  body: RiskActionRequest,
) {
  return postJson<RiskActionRequest, RiskActionResult>(
    credentials,
    `/workspaces/${credentials.workspaceId}/admin/risk/policies/${policyId}/block`,
    body,
  );
}

export function resolveRiskSignal(
  credentials: AdminCredentials,
  signalId: string,
  body: RiskActionRequest,
) {
  return postJson<RiskActionRequest, RiskActionResult>(
    credentials,
    `/workspaces/${credentials.workspaceId}/admin/risk/signals/${signalId}/resolve`,
    body,
  );
}

export async function getBillingPlatformCenter(credentials: AdminCredentials) {
  const response = await verifiedFetch('/admin/billing-platform-center', {
    headers: authHeaders(credentials),
  });
  return parseJson<BillingPlatformSnapshot>(response);
}

export async function getCustomerCenter(credentials: AdminCredentials) {
  const response = await verifiedFetch('/admin/customer-center', { headers: authHeaders(credentials) });
  return parseJson<CustomerCenterSnapshot>(response);
}

export async function getDeveloperCenter(credentials: AdminCredentials) {
  const response = await verifiedFetch('/admin/developer-center', { headers: authHeaders(credentials) });
  return parseJson<DeveloperCenterSnapshot>(response);
}

export function revokeDeveloperClient(
  credentials: AdminCredentials,
  clientId: string,
  body: DeveloperActionRequest,
) {
  return postJson<DeveloperActionRequest, DeveloperActionResult>(
    credentials,
    `/workspaces/${credentials.workspaceId}/admin/developer/clients/${clientId}/revoke`,
    body,
  );
}

export function rotateDeveloperSecret(
  credentials: AdminCredentials,
  clientId: string,
  body: DeveloperActionRequest,
) {
  return postJson<DeveloperActionRequest, DeveloperActionResult>(
    credentials,
    `/workspaces/${credentials.workspaceId}/admin/developer/clients/${clientId}/rotate-secret`,
    body,
  );
}

export function approveMarketplaceApp(
  credentials: AdminCredentials,
  appId: string,
  body: DeveloperActionRequest,
) {
  return postJson<DeveloperActionRequest, DeveloperActionResult>(
    credentials,
    `/workspaces/${credentials.workspaceId}/admin/developer/marketplace-apps/${appId}/approve`,
    body,
  );
}

export async function getEntitlementsCenter(credentials: AdminCredentials) {
  const response = await verifiedFetch('/admin/entitlements-center', {
    headers: authHeaders(credentials),
  });
  return parseJson<EntitlementsSnapshot>(response);
}

export function grantEntitlementFeature(
  credentials: AdminCredentials,
  body: EntitlementFeatureActionRequest,
) {
  return postJson<EntitlementFeatureActionRequest, EntitlementActionResult>(
    credentials,
    `/workspaces/${credentials.workspaceId}/admin/entitlements/grants`,
    body,
  );
}

export function revokeEntitlementFeature(
  credentials: AdminCredentials,
  body: EntitlementFeatureActionRequest,
) {
  return postJson<EntitlementFeatureActionRequest, EntitlementActionResult>(
    credentials,
    `/workspaces/${credentials.workspaceId}/admin/entitlements/revocations`,
    body,
  );
}

export function overrideEntitlementQuota(
  credentials: AdminCredentials,
  body: EntitlementQuotaOverrideRequest,
) {
  return postJson<EntitlementQuotaOverrideRequest, EntitlementActionResult>(
    credentials,
    `/workspaces/${credentials.workspaceId}/admin/entitlements/quota-overrides`,
    body,
  );
}

export function publishEntitlementChanges(
  credentials: AdminCredentials,
  body: EntitlementPublishRequest,
) {
  return postJson<EntitlementPublishRequest, EntitlementActionResult>(
    credentials,
    `/workspaces/${credentials.workspaceId}/admin/entitlements/publish`,
    body,
  );
}

export async function getIdentityGovernanceCenter(credentials: AdminCredentials) {
  const response = await verifiedFetch('/admin/identity-governance-center', {
    headers: authHeaders(credentials),
  });
  return parseJson<IdentityGovernanceSnapshot>(response);
}

export function revokeBreakGlassAccount(
  credentials: AdminCredentials,
  tenantId: string,
  principalId: string,
  body: GovernanceActionRequest,
) {
  return postJson<GovernanceActionRequest, GovernanceActionResult>(
    credentials,
    `/admin/identity-governance-center/break-glass/${tenantId}/${principalId}/revoke`,
    body,
  );
}

export function cancelRecoveryRequest(
  credentials: AdminCredentials,
  requestId: string,
  body: GovernanceActionRequest,
) {
  return postJson<GovernanceActionRequest, GovernanceActionResult>(
    credentials,
    `/admin/identity-governance-center/recovery-requests/${requestId}/cancel`,
    body,
  );
}

export async function getOperationsCenter(credentials: AdminCredentials) {
  const response = await verifiedFetch('/admin/operations-center', { headers: authHeaders(credentials) });
  return parseJson<OperationsCenterSnapshot>(response);
}

export function replayOperationsProviderEvent(
  credentials: AdminCredentials,
  eventId: string,
  body: OperationsActionRequest,
) {
  return postJson<OperationsActionRequest, OperationsActionResult>(
    credentials,
    `/workspaces/${credentials.workspaceId}/admin/operations/provider-events/${eventId}/replay`,
    body,
  );
}

export function replayExportRun(
  credentials: AdminCredentials,
  exportRunId: string,
  body: OperationsActionRequest,
) {
  return postJson<OperationsActionRequest, OperationsActionResult>(
    credentials,
    `/workspaces/${credentials.workspaceId}/admin/operations/export-runs/${exportRunId}/replay`,
    body,
  );
}

export function resolveReconciliationDifference(
  credentials: AdminCredentials,
  differenceId: string,
  body: OperationsActionRequest,
) {
  return postJson<OperationsActionRequest, OperationsActionResult>(
    credentials,
    `/workspaces/${credentials.workspaceId}/admin/operations/reconciliation-differences/${differenceId}/resolve`,
    body,
  );
}

export async function getUsageCenter(credentials: AdminCredentials) {
  const response = await verifiedFetch('/admin/usage-center', { headers: authHeaders(credentials) });
  return parseJson<UsageCenterSnapshot>(response);
}

export function correctUsage(credentials: AdminCredentials, body: UsageCorrectionRequest) {
  return postJson<UsageCorrectionRequest, UsageActionResult>(
    credentials,
    `/workspaces/${credentials.workspaceId}/admin/usage/corrections`,
    body,
  );
}

export function freezeUsageMeter(credentials: AdminCredentials, body: FreezeMeterRequest) {
  return postJson<FreezeMeterRequest, UsageActionResult>(
    credentials,
    `/workspaces/${credentials.workspaceId}/admin/usage/meters/freeze`,
    body,
  );
}

export function replayUsageRollup(
  credentials: AdminCredentials,
  rollupId: string,
  body: ReplayUsageRollupRequest,
) {
  return postJson<ReplayUsageRollupRequest, UsageActionResult>(
    credentials,
    `/workspaces/${credentials.workspaceId}/admin/usage/rollups/${rollupId}/replay`,
    body,
  );
}

export async function globalSearch(credentials: AdminCredentials, query: string) {
  const params = new URLSearchParams({ q: query });
  const response = await verifiedFetch(`/admin/search?${params.toString()}`, {
    headers: authHeaders(credentials),
  });
  return parseJson<GlobalSearchResult[]>(response);
}

export async function getTenantDetail(credentials: AdminCredentials, tenantId: string) {
  const response = await verifiedFetch(`/admin/tenants/${tenantId}`, {
    headers: authHeaders(credentials),
  });
  return parseJson<TenantDetail>(response);
}

export function suspendTenant(
  credentials: AdminCredentials,
  tenantId: string,
  body: TenantLifecycleRequest,
) {
  return postJson<TenantLifecycleRequest, TenantLifecycleResult>(
    credentials,
    `/admin/tenants/${tenantId}/suspend`,
    body,
  );
}

export function reactivateTenant(
  credentials: AdminCredentials,
  tenantId: string,
  body: TenantLifecycleRequest,
) {
  return postJson<TenantLifecycleRequest, TenantLifecycleResult>(
    credentials,
    `/admin/tenants/${tenantId}/reactivate`,
    body,
  );
}

export async function getWorkspaceDetail(credentials: AdminCredentials, workspaceId: string) {
  const response = await verifiedFetch(`/admin/workspaces/${workspaceId}`, {
    headers: authHeaders(credentials),
  });
  return parseJson<WorkspaceDetail>(response);
}

export function suspendWorkspace(
  credentials: AdminCredentials,
  workspaceId: string,
  body: WorkspaceLifecycleRequest,
) {
  return postJson<WorkspaceLifecycleRequest, WorkspaceLifecycleResult>(
    credentials,
    `/admin/workspaces/${workspaceId}/suspend`,
    body,
  );
}

export function reactivateWorkspace(
  credentials: AdminCredentials,
  workspaceId: string,
  body: WorkspaceLifecycleRequest,
) {
  return postJson<WorkspaceLifecycleRequest, WorkspaceLifecycleResult>(
    credentials,
    `/admin/workspaces/${workspaceId}/reactivate`,
    body,
  );
}

export async function getUserDetail(credentials: AdminCredentials, principalId: string) {
  const response = await verifiedFetch(`/admin/users/${principalId}`, {
    headers: authHeaders(credentials),
  });
  return parseJson<UserDetail>(response);
}

export function suspendUser(
  credentials: AdminCredentials,
  principalId: string,
  body: UserLifecycleRequest,
) {
  return postJson<UserLifecycleRequest, UserLifecycleResult>(
    credentials,
    `/admin/users/${principalId}/suspend`,
    body,
  );
}

export function reactivateUser(
  credentials: AdminCredentials,
  principalId: string,
  body: UserLifecycleRequest,
) {
  return postJson<UserLifecycleRequest, UserLifecycleResult>(
    credentials,
    `/admin/users/${principalId}/reactivate`,
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
