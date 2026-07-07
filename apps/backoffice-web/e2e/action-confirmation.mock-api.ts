import type { Page } from '@playwright/test';
import * as ids from './action-confirmation.fixtures';
import { pendingApprovalsSnapshot } from './action-confirmation.snapshots.approvals';
import {
  auditEventsSnapshot,
  auditEvidenceExportSnapshot,
  auditEvidenceSnapshot,
  billingOverview,
  billingPlatformSnapshot,
  billingRunbooksSnapshot,
  commandSnapshot,
  communicationsSnapshot,
  complianceSnapshot,
  customerSnapshot,
  developerSnapshot,
  entitlementsSnapshot,
  operationsSnapshot,
  regionSnapshot,
  revenueSnapshot,
  riskDecisionSnapshot,
  usageSnapshot,
} from './action-confirmation.snapshots.centers';
import {
  globalSearchSnapshot,
  resultForMutation,
  tenantDetailSnapshot,
  userDetailSnapshot,
  workspaceDetailSnapshot,
} from './action-confirmation.snapshots.detail';
import {
  accessSnapshot,
  governanceSnapshot,
  securitySnapshot,
} from './action-confirmation.snapshots.identity';

export type MutationPost = {
  backofficeRole: string;
  confirmCode: string;
  hasIdempotencyKey: boolean;
  path: string;
  secondApproverPrincipalId: string;
  secondApproverRole: string;
};

export async function installCredentials(page: Page, backofficeRole = 'platform_admin') {
  await page.addInitScript(
    ({ actorIdValue, backofficeRoleValue, secondApproverPrincipalIdValue, tenantWorkspaceId }) => {
      window.localStorage.setItem(
        'nvbes.backoffice-service.credentials',
        JSON.stringify({
          workspaceId: tenantWorkspaceId,
          internalToken: 'test-internal-token',
          actorPrincipalId: actorIdValue,
          backofficeRole: backofficeRoleValue,
          secondApproverPrincipalId: secondApproverPrincipalIdValue,
          secondApproverRole: 'platform_admin',
        }),
      );
    },
    {
      actorIdValue: ids.actorId,
      backofficeRoleValue: backofficeRole,
      secondApproverPrincipalIdValue: ids.secondApproverId,
      tenantWorkspaceId: ids.workspaceId,
    },
  );
}

export async function installMockApi(page: Page) {
  await page.route('**/*', async (route) => {
    const request = route.request();
    const url = new URL(request.url());
    if (!url.pathname.startsWith('/admin') && !url.pathname.startsWith('/workspaces')) {
      await route.continue();
      return;
    }

    if (request.method() === 'GET') {
      await route.fulfill({
        contentType: 'application/json',
        body: JSON.stringify(snapshotForPath(url.pathname)),
      });
      return;
    }

    await route.continue();
  });
}

export async function captureMutation(page: Page, posts: MutationPost[]) {
  await page.route('**/*', async (route) => {
    const request = route.request();
    const url = new URL(request.url());
    const isBackofficeMutation =
      url.pathname.startsWith('/admin') || url.pathname.startsWith('/workspaces');
    if (request.method() !== 'POST' || !isBackofficeMutation) {
      await route.fallback();
      return;
    }
    const payload = request.postDataJSON() as { confirm_code?: string };
    posts.push({
      backofficeRole: request.headers()['x-nvbes-backoffice-role'] ?? '',
      confirmCode: payload.confirm_code ?? '',
      hasIdempotencyKey: Boolean(request.headers()['idempotency-key']),
      path: url.pathname,
      secondApproverPrincipalId: request.headers()['x-nvbes-second-approver-principal-id'] ?? '',
      secondApproverRole: request.headers()['x-nvbes-second-approver-role'] ?? '',
    });
    await route.fulfill({
      contentType: 'application/json',
      body: JSON.stringify(resultForMutation(url.pathname)),
    });
  });
}

export async function rejectMutations(page: Page, posts: MutationPost[], message: string) {
  await page.route('**/*', async (route) => {
    const request = route.request();
    const url = new URL(request.url());
    const isBackofficeMutation =
      url.pathname.startsWith('/admin') || url.pathname.startsWith('/workspaces');
    if (request.method() !== 'POST' || !isBackofficeMutation) {
      await route.fallback();
      return;
    }
    const payload = request.postDataJSON() as { confirm_code?: string };
    posts.push({
      backofficeRole: request.headers()['x-nvbes-backoffice-role'] ?? '',
      confirmCode: payload.confirm_code ?? '',
      hasIdempotencyKey: Boolean(request.headers()['idempotency-key']),
      path: url.pathname,
      secondApproverPrincipalId: request.headers()['x-nvbes-second-approver-principal-id'] ?? '',
      secondApproverRole: request.headers()['x-nvbes-second-approver-role'] ?? '',
    });
    await route.fulfill({
      body: message,
      contentType: 'text/plain',
      status: 403,
    });
  });
}

export async function openDetailFromGlobalSearch(page: Page, query: string, buttonName: string) {
  await page.goto('/#global-search');
  const section = page.locator('section#global-search');
  await section.getByPlaceholder('tenant slug, workspace, email, UUID...').fill(query);
  await section.getByRole('button', { name: 'Search' }).click();
  await section.getByRole('button', { name: buttonName }).first().click();
}

function snapshotForPath(path: string) {
  if (path === '/admin/command-center') return commandSnapshot();
  if (path === '/admin/pending-approvals') return pendingApprovalsSnapshot();
  if (path === '/admin/access-center') return accessSnapshot();
  if (path === '/admin/audit-evidence-center') return auditEvidenceSnapshot();
  if (path === '/admin/billing-platform-center') return billingPlatformSnapshot();
  if (path === '/admin/communications-center') return communicationsSnapshot();
  if (path === '/admin/compliance-center') return complianceSnapshot();
  if (path === '/admin/customer-center') return customerSnapshot();
  if (path === '/admin/developer-center') return developerSnapshot();
  if (path === '/admin/entitlements-center') return entitlementsSnapshot();
  if (path === '/admin/operations-center') return operationsSnapshot();
  if (path === '/admin/region-center') return regionSnapshot();
  if (path === '/admin/revenue-center') return revenueSnapshot();
  if (path === '/admin/risk-decision-center') return riskDecisionSnapshot();
  if (path === '/admin/security-center') return securitySnapshot();
  if (path === '/admin/usage-center') return usageSnapshot();
  if (path === '/admin/identity-governance-center') return governanceSnapshot();
  if (path === '/admin/search') return globalSearchSnapshot();
  if (path === `/admin/tenants/${ids.tenantId}`) return tenantDetailSnapshot();
  if (path === `/admin/workspaces/${ids.workspaceId}`) return workspaceDetailSnapshot();
  if (path === `/admin/users/${ids.principalId}`) return userDetailSnapshot();
  if (path.endsWith('/admin/audit-evidence/export')) return auditEvidenceExportSnapshot();
  if (path === '/admin/audit-events' || path.endsWith('/admin/audit-events')) {
    return auditEventsSnapshot();
  }
  if (path === `/workspaces/${ids.workspaceId}/billing/admin/overview`) return billingOverview();
  if (path === `/workspaces/${ids.workspaceId}/billing/admin/provider-events/failures`) return [];
  if (path === '/admin/billing/runbooks') return billingRunbooksSnapshot();
  return {};
}
