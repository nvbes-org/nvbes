import { verifiedFetch } from '@nvbes/web-runtime';
import type {
  AdminCredentials,
  CustomerCenterSnapshot,
  DeveloperCenterSnapshot,
  EntitlementsSnapshot,
  OperationsCenterSnapshot,
  UsageCenterSnapshot,
} from './backoffice-service.types';
import type { OperationBody, OperationOk, OperationPath } from './backoffice-service.generated-api';
import { authHeaders, parseJson, postJson } from './backoffice-service.api.core';

export async function getCustomerCenter(credentials: AdminCredentials) {
  const response = await verifiedFetch('/admin/customer-center', {
    headers: authHeaders(credentials),
  });
  return parseJson<CustomerCenterSnapshot>(response);
}

export async function getDeveloperCenter(credentials: AdminCredentials) {
  const response = await verifiedFetch('/admin/developer-center', {
    headers: authHeaders(credentials),
  });
  return parseJson<DeveloperCenterSnapshot>(response);
}

export function revokeDeveloperClient(
  credentials: AdminCredentials,
  clientId: OperationPath<'revokeDeveloperClient'>['clientId'],
  body: OperationBody<'revokeDeveloperClient'>,
) {
  return postJson<OperationBody<'revokeDeveloperClient'>, OperationOk<'revokeDeveloperClient'>>(
    credentials,
    `/workspaces/${credentials.workspaceId}/admin/developer/clients/${clientId}/revoke`,
    body,
  );
}

export function rotateDeveloperSecret(
  credentials: AdminCredentials,
  clientId: OperationPath<'rotateDeveloperSecret'>['clientId'],
  body: OperationBody<'rotateDeveloperSecret'>,
) {
  return postJson<OperationBody<'rotateDeveloperSecret'>, OperationOk<'rotateDeveloperSecret'>>(
    credentials,
    `/workspaces/${credentials.workspaceId}/admin/developer/clients/${clientId}/rotate-secret`,
    body,
  );
}

export function approveMarketplaceApp(
  credentials: AdminCredentials,
  appId: OperationPath<'approveMarketplaceApp'>['appId'],
  body: OperationBody<'approveMarketplaceApp'>,
) {
  return postJson<OperationBody<'approveMarketplaceApp'>, OperationOk<'approveMarketplaceApp'>>(
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
  body: OperationBody<'grantEntitlementFeature'>,
) {
  return postJson<OperationBody<'grantEntitlementFeature'>, OperationOk<'grantEntitlementFeature'>>(
    credentials,
    `/workspaces/${credentials.workspaceId}/admin/entitlements/grants`,
    body,
  );
}

export function revokeEntitlementFeature(
  credentials: AdminCredentials,
  body: OperationBody<'revokeEntitlementFeature'>,
) {
  return postJson<
    OperationBody<'revokeEntitlementFeature'>,
    OperationOk<'revokeEntitlementFeature'>
  >(credentials, `/workspaces/${credentials.workspaceId}/admin/entitlements/revocations`, body);
}

export function overrideEntitlementQuota(
  credentials: AdminCredentials,
  body: OperationBody<'overrideEntitlementQuota'>,
) {
  return postJson<
    OperationBody<'overrideEntitlementQuota'>,
    OperationOk<'overrideEntitlementQuota'>
  >(credentials, `/workspaces/${credentials.workspaceId}/admin/entitlements/quota-overrides`, body);
}

export function publishEntitlementChanges(
  credentials: AdminCredentials,
  body: OperationBody<'publishEntitlementChanges'>,
) {
  return postJson<
    OperationBody<'publishEntitlementChanges'>,
    OperationOk<'publishEntitlementChanges'>
  >(credentials, `/workspaces/${credentials.workspaceId}/admin/entitlements/publish`, body);
}

export async function getOperationsCenter(credentials: AdminCredentials) {
  const response = await verifiedFetch('/admin/operations-center', {
    headers: authHeaders(credentials),
  });
  return parseJson<OperationsCenterSnapshot>(response);
}

export function replayOperationsProviderEvent(
  credentials: AdminCredentials,
  eventId: OperationPath<'replayOperationsProviderEvent'>['eventId'],
  body: OperationBody<'replayOperationsProviderEvent'>,
) {
  return postJson<
    OperationBody<'replayOperationsProviderEvent'>,
    OperationOk<'replayOperationsProviderEvent'>
  >(
    credentials,
    `/workspaces/${credentials.workspaceId}/admin/operations/provider-events/${eventId}/replay`,
    body,
  );
}

export function updateOperationsIncidentState(
  credentials: AdminCredentials,
  incidentId: OperationPath<'updateOperationsIncidentState'>['incidentId'],
  body: OperationBody<'updateOperationsIncidentState'>,
) {
  return postJson<
    OperationBody<'updateOperationsIncidentState'>,
    OperationOk<'updateOperationsIncidentState'>
  >(
    credentials,
    `/workspaces/${credentials.workspaceId}/admin/operations/incidents/${incidentId}/state`,
    body,
  );
}

export function scheduleMaintenanceWindow(
  credentials: AdminCredentials,
  body: OperationBody<'scheduleMaintenanceWindow'>,
) {
  return postJson<
    OperationBody<'scheduleMaintenanceWindow'>,
    OperationOk<'scheduleMaintenanceWindow'>
  >(
    credentials,
    `/workspaces/${credentials.workspaceId}/admin/operations/maintenance-windows`,
    body,
  );
}

export function replayOperationsJobRun(
  credentials: AdminCredentials,
  jobRunId: OperationPath<'replayOperationsJobRun'>['jobRunId'],
  body: OperationBody<'replayOperationsJobRun'>,
) {
  return postJson<OperationBody<'replayOperationsJobRun'>, OperationOk<'replayOperationsJobRun'>>(
    credentials,
    `/workspaces/${credentials.workspaceId}/admin/operations/job-runs/${jobRunId}/replay`,
    body,
  );
}

export function replayExportRun(
  credentials: AdminCredentials,
  exportRunId: OperationPath<'replayExportRun'>['exportRunId'],
  body: OperationBody<'replayExportRun'>,
) {
  return postJson<OperationBody<'replayExportRun'>, OperationOk<'replayExportRun'>>(
    credentials,
    `/workspaces/${credentials.workspaceId}/admin/operations/export-runs/${exportRunId}/replay`,
    body,
  );
}

export function resolveReconciliationDifference(
  credentials: AdminCredentials,
  differenceId: OperationPath<'resolveReconciliationDifference'>['differenceId'],
  body: OperationBody<'resolveReconciliationDifference'>,
) {
  return postJson<
    OperationBody<'resolveReconciliationDifference'>,
    OperationOk<'resolveReconciliationDifference'>
  >(
    credentials,
    `/workspaces/${credentials.workspaceId}/admin/operations/reconciliation-differences/${differenceId}/resolve`,
    body,
  );
}

export async function getUsageCenter(credentials: AdminCredentials) {
  const response = await verifiedFetch('/admin/usage-center', {
    headers: authHeaders(credentials),
  });
  return parseJson<UsageCenterSnapshot>(response);
}

export function correctUsage(credentials: AdminCredentials, body: OperationBody<'correctUsage'>) {
  return postJson<OperationBody<'correctUsage'>, OperationOk<'correctUsage'>>(
    credentials,
    `/workspaces/${credentials.workspaceId}/admin/usage/corrections`,
    body,
  );
}

export function freezeUsageMeter(
  credentials: AdminCredentials,
  body: OperationBody<'freezeUsageMeter'>,
) {
  return postJson<OperationBody<'freezeUsageMeter'>, OperationOk<'freezeUsageMeter'>>(
    credentials,
    `/workspaces/${credentials.workspaceId}/admin/usage/meters/freeze`,
    body,
  );
}

export function replayUsageRollup(
  credentials: AdminCredentials,
  rollupId: OperationPath<'replayUsageRollup'>['rollupId'],
  body: OperationBody<'replayUsageRollup'>,
) {
  return postJson<OperationBody<'replayUsageRollup'>, OperationOk<'replayUsageRollup'>>(
    credentials,
    `/workspaces/${credentials.workspaceId}/admin/usage/rollups/${rollupId}/replay`,
    body,
  );
}
