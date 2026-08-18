import { verifiedFetch } from '@nvbes/web-runtime';
import type {
  AdminCredentials,
  CommunicationsCenterSnapshot,
  ComplianceCenterSnapshot,
  RegionCenterSnapshot,
  RiskDecisionSnapshot,
} from './backoffice-service.types';
import type { OperationBody, OperationOk, OperationPath } from './backoffice-service.generated-api';
import { authHeaders, parseJson, postJson } from './backoffice-service.api.core';

export async function getComplianceCenter(credentials: AdminCredentials) {
  const response = await verifiedFetch('/admin/compliance-center', {
    headers: authHeaders(credentials),
  });
  return parseJson<ComplianceCenterSnapshot>(response);
}

export function revokeComplianceConsent(
  credentials: AdminCredentials,
  consentId: OperationPath<'revokeConsent'>['consentId'],
  body: OperationBody<'revokeConsent'>,
) {
  return postJson<OperationBody<'revokeConsent'>, OperationOk<'revokeConsent'>>(
    credentials,
    `/workspaces/${credentials.workspaceId}/admin/compliance/consents/${consentId}/revoke`,
    body,
  );
}

export function requestComplianceErasure(
  credentials: AdminCredentials,
  principalId: OperationPath<'requestComplianceErasure'>['principalId'],
  body: OperationBody<'requestComplianceErasure'>,
) {
  return postJson<
    OperationBody<'requestComplianceErasure'>,
    OperationOk<'requestComplianceErasure'>
  >(
    credentials,
    `/workspaces/${credentials.workspaceId}/admin/compliance/principals/${principalId}/erasure-request`,
    body,
  );
}

export function reviewComplianceSuppression(
  credentials: AdminCredentials,
  body: OperationBody<'reviewComplianceSuppression'>,
) {
  return postJson<
    OperationBody<'reviewComplianceSuppression'>,
    OperationOk<'reviewComplianceSuppression'>
  >(
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
  messageId: OperationPath<'replayEmailMessage'>['messageId'],
  body: OperationBody<'replayEmailMessage'>,
) {
  return postJson<OperationBody<'replayEmailMessage'>, OperationOk<'replayEmailMessage'>>(
    credentials,
    `/workspaces/${credentials.workspaceId}/admin/communications/emails/${messageId}/replay`,
    body,
  );
}

export function suppressEmail(credentials: AdminCredentials, body: OperationBody<'suppressEmail'>) {
  return postJson<OperationBody<'suppressEmail'>, OperationOk<'suppressEmail'>>(
    credentials,
    `/workspaces/${credentials.workspaceId}/admin/communications/suppressions`,
    body,
  );
}

export function unsuppressEmail(
  credentials: AdminCredentials,
  body: OperationBody<'unsuppressEmail'>,
) {
  return postJson<OperationBody<'unsuppressEmail'>, OperationOk<'unsuppressEmail'>>(
    credentials,
    `/workspaces/${credentials.workspaceId}/admin/communications/suppressions/remove`,
    body,
  );
}

export async function getRegionCenter(credentials: AdminCredentials) {
  const response = await verifiedFetch('/admin/region-center', {
    headers: authHeaders(credentials),
  });
  return parseJson<RegionCenterSnapshot>(response);
}

export function flagRegionResidency(
  credentials: AdminCredentials,
  targetWorkspaceId: OperationPath<'flagWorkspaceResidency'>['targetWorkspaceId'],
  body: OperationBody<'flagWorkspaceResidency'>,
) {
  return postJson<OperationBody<'flagWorkspaceResidency'>, OperationOk<'flagWorkspaceResidency'>>(
    credentials,
    `/workspaces/${credentials.workspaceId}/admin/region/workspaces/${targetWorkspaceId}/residency-flag`,
    body,
  );
}

export function recordRegionException(
  credentials: AdminCredentials,
  targetWorkspaceId: OperationPath<'recordRegionException'>['targetWorkspaceId'],
  body: OperationBody<'recordRegionException'>,
) {
  return postJson<OperationBody<'recordRegionException'>, OperationOk<'recordRegionException'>>(
    credentials,
    `/workspaces/${credentials.workspaceId}/admin/region/workspaces/${targetWorkspaceId}/exceptions`,
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
  policyId: OperationPath<'approveRiskPolicy'>['policyId'],
  body: OperationBody<'approveRiskPolicy'>,
) {
  return postJson<OperationBody<'approveRiskPolicy'>, OperationOk<'approveRiskPolicy'>>(
    credentials,
    `/workspaces/${credentials.workspaceId}/admin/risk/policies/${policyId}/approve`,
    body,
  );
}

export function blockRiskPolicy(
  credentials: AdminCredentials,
  policyId: OperationPath<'blockRiskPolicy'>['policyId'],
  body: OperationBody<'blockRiskPolicy'>,
) {
  return postJson<OperationBody<'blockRiskPolicy'>, OperationOk<'blockRiskPolicy'>>(
    credentials,
    `/workspaces/${credentials.workspaceId}/admin/risk/policies/${policyId}/block`,
    body,
  );
}

export function resolveRiskSignal(
  credentials: AdminCredentials,
  signalId: OperationPath<'resolveRiskSignal'>['signalId'],
  body: OperationBody<'resolveRiskSignal'>,
) {
  return postJson<OperationBody<'resolveRiskSignal'>, OperationOk<'resolveRiskSignal'>>(
    credentials,
    `/workspaces/${credentials.workspaceId}/admin/risk/signals/${signalId}/resolve`,
    body,
  );
}
