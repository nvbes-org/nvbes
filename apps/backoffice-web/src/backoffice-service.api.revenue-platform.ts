import { verifiedFetch } from '@nvbes/web-runtime';
import type {
  AdminCredentials,
  BillingPlatformSnapshot,
  RevenueCenterSnapshot,
} from './backoffice-service.types';
import type { OperationBody, OperationOk, OperationPath } from './backoffice-service.generated-api';
import { authHeaders, parseJson, postJson } from './backoffice-service.api.core';

export async function getRevenueCenter(credentials: AdminCredentials) {
  const response = await verifiedFetch('/admin/revenue-center', {
    headers: authHeaders(credentials),
  });
  return parseJson<RevenueCenterSnapshot>(response);
}

export function closeDunningCase(
  credentials: AdminCredentials,
  caseId: OperationPath<'closeDunningCase'>['caseId'],
  body: OperationBody<'closeDunningCase'>,
) {
  return postJson<OperationBody<'closeDunningCase'>, OperationOk<'closeDunningCase'>>(
    credentials,
    `/workspaces/${credentials.workspaceId}/admin/revenue/dunning-cases/${caseId}/close`,
    body,
  );
}

export function reopenDunningCase(
  credentials: AdminCredentials,
  caseId: OperationPath<'reopenDunningCase'>['caseId'],
  body: OperationBody<'reopenDunningCase'>,
) {
  return postJson<OperationBody<'reopenDunningCase'>, OperationOk<'reopenDunningCase'>>(
    credentials,
    `/workspaces/${credentials.workspaceId}/admin/revenue/dunning-cases/${caseId}/reopen`,
    body,
  );
}

export function holdRevenueInvoice(
  credentials: AdminCredentials,
  invoiceId: OperationPath<'holdRevenueInvoice'>['invoiceId'],
  body: OperationBody<'holdRevenueInvoice'>,
) {
  return postJson<OperationBody<'holdRevenueInvoice'>, OperationOk<'holdRevenueInvoice'>>(
    credentials,
    `/workspaces/${credentials.workspaceId}/admin/revenue/invoices/${invoiceId}/hold`,
    body,
  );
}

export function releaseRevenueInvoice(
  credentials: AdminCredentials,
  invoiceId: OperationPath<'releaseRevenueInvoice'>['invoiceId'],
  body: OperationBody<'releaseRevenueInvoice'>,
) {
  return postJson<OperationBody<'releaseRevenueInvoice'>, OperationOk<'releaseRevenueInvoice'>>(
    credentials,
    `/workspaces/${credentials.workspaceId}/admin/revenue/invoices/${invoiceId}/release`,
    body,
  );
}

export function reviewRevenueDispute(
  credentials: AdminCredentials,
  disputeId: OperationPath<'reviewRevenueDispute'>['disputeId'],
  body: OperationBody<'reviewRevenueDispute'>,
) {
  return postJson<OperationBody<'reviewRevenueDispute'>, OperationOk<'reviewRevenueDispute'>>(
    credentials,
    `/workspaces/${credentials.workspaceId}/admin/revenue/disputes/${disputeId}/review`,
    body,
  );
}

export function resolveRevenueDispute(
  credentials: AdminCredentials,
  disputeId: OperationPath<'resolveRevenueDispute'>['disputeId'],
  body: OperationBody<'resolveRevenueDispute'>,
) {
  return postJson<OperationBody<'resolveRevenueDispute'>, OperationOk<'resolveRevenueDispute'>>(
    credentials,
    `/workspaces/${credentials.workspaceId}/admin/revenue/disputes/${disputeId}/resolve`,
    body,
  );
}

export async function getBillingPlatformCenter(credentials: AdminCredentials) {
  const response = await verifiedFetch('/admin/billing-platform-center', {
    headers: authHeaders(credentials),
  });
  return parseJson<BillingPlatformSnapshot>(response);
}

export function enableProviderRoutingRule(
  credentials: AdminCredentials,
  ruleId: OperationPath<'enableBillingRoutingRule'>['ruleId'],
  body: OperationBody<'enableBillingRoutingRule'>,
) {
  return postJson<
    OperationBody<'enableBillingRoutingRule'>,
    OperationOk<'enableBillingRoutingRule'>
  >(
    credentials,
    `/workspaces/${credentials.workspaceId}/admin/billing-platform/routing-rules/${ruleId}/enable`,
    body,
  );
}

export function disableProviderRoutingRule(
  credentials: AdminCredentials,
  ruleId: OperationPath<'disableBillingRoutingRule'>['ruleId'],
  body: OperationBody<'disableBillingRoutingRule'>,
) {
  return postJson<
    OperationBody<'disableBillingRoutingRule'>,
    OperationOk<'disableBillingRoutingRule'>
  >(
    credentials,
    `/workspaces/${credentials.workspaceId}/admin/billing-platform/routing-rules/${ruleId}/disable`,
    body,
  );
}

export function approveKycProfile(
  credentials: AdminCredentials,
  profileId: OperationPath<'approveKycProfile'>['profileId'],
  body: OperationBody<'approveKycProfile'>,
) {
  return postJson<OperationBody<'approveKycProfile'>, OperationOk<'approveKycProfile'>>(
    credentials,
    `/workspaces/${credentials.workspaceId}/admin/billing-platform/kyc-profiles/${profileId}/approve`,
    body,
  );
}

export function rejectKycProfile(
  credentials: AdminCredentials,
  profileId: OperationPath<'rejectKycProfile'>['profileId'],
  body: OperationBody<'rejectKycProfile'>,
) {
  return postJson<OperationBody<'rejectKycProfile'>, OperationOk<'rejectKycProfile'>>(
    credentials,
    `/workspaces/${credentials.workspaceId}/admin/billing-platform/kyc-profiles/${profileId}/reject`,
    body,
  );
}

export function activateEinvoicingProfile(
  credentials: AdminCredentials,
  profileId: OperationPath<'activateEinvoicingProfile'>['profileId'],
  body: OperationBody<'activateEinvoicingProfile'>,
) {
  return postJson<
    OperationBody<'activateEinvoicingProfile'>,
    OperationOk<'activateEinvoicingProfile'>
  >(
    credentials,
    `/workspaces/${credentials.workspaceId}/admin/billing-platform/einvoicing-profiles/${profileId}/activate`,
    body,
  );
}
