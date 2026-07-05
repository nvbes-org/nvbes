import { expect, test } from '@playwright/test';
import * as ids from './action-confirmation.fixtures';
import {
  captureMutation,
  installCredentials,
  installMockApi,
  openDetailFromGlobalSearch,
  type MutationPost,
} from './action-confirmation.mock-api';

test.describe('backoffice-service compliance region and risk action flows @mocked', () => {
  test.beforeEach(async ({ page }) => {
    await installCredentials(page);
    await installMockApi(page);
  });

  test('Compliance Center requires principal-bound erasure confirmation', async ({ page }) => {
    const posts: MutationPost[] = [];
    await captureMutation(page, posts);

    await page.goto('/');
    const section = page.locator('section#compliance-center');
    await section.getByRole('button', { name: 'Erasure' }).click();
    const execute = section.getByRole('button', { name: 'Execute action' });

    await section.getByLabel('Principal ID').fill(ids.principalId);
    await section.getByLabel('Reason').fill('ticket GDPR-123 approved');
    await section.getByLabel('Confirmation code').fill('REQUEST ERASURE');
    await expect(execute).toBeDisabled();

    await section.getByLabel('Confirmation code').fill('REQUEST ERASURE 11111111');
    await expect(execute).toBeEnabled();
    await execute.click();

    await expect(
      page.getByText(`internal_admin.compliance.erasure.requested - queued - ${ids.principalId}`),
    ).toBeVisible();
    expect(posts).toContainEqual(
      expect.objectContaining({
        confirmCode: 'REQUEST ERASURE 11111111',
        hasIdempotencyKey: true,
        path: `/workspaces/${ids.workspaceId}/admin/compliance/principals/${ids.principalId}/erasure-request`,
        secondApproverPrincipalId: ids.secondApproverId,
        secondApproverRole: 'platform_admin',
      }),
    );
  });

  test('Compliance Center requires consent-bound revoke confirmation', async ({ page }) => {
    const posts: MutationPost[] = [];
    await captureMutation(page, posts);

    await page.goto('/');
    const section = page.locator('section#compliance-center');
    const execute = section.getByRole('button', { name: 'Execute action' });

    await section.getByLabel('Consent ID').fill(ids.consentId);
    await section.getByLabel('Reason').fill('ticket GDPR-456 approved');
    await section.getByLabel('Confirmation code').fill('REVOKE CONSENT');
    await expect(execute).toBeDisabled();

    await section.getByLabel('Confirmation code').fill('REVOKE CONSENT ABABABAB');
    await expect(execute).toBeEnabled();
    await execute.click();

    await expect(
      page.getByText(`internal_admin.compliance.consent.revoked - applied - ${ids.consentId}`),
    ).toBeVisible();
    expect(posts).toContainEqual(
      expect.objectContaining({
        confirmCode: 'REVOKE CONSENT ABABABAB',
        hasIdempotencyKey: true,
        path: `/workspaces/${ids.workspaceId}/admin/compliance/consents/${ids.consentId}/revoke`,
        secondApproverPrincipalId: ids.secondApproverId,
        secondApproverRole: 'platform_admin',
      }),
    );
  });

  test('Region Center requires workspace-bound exception confirmation', async ({ page }) => {
    const posts: MutationPost[] = [];
    await captureMutation(page, posts);

    await page.goto('/');
    const section = page.locator('section#region-center');
    await section.getByRole('button', { name: 'Exception' }).click();
    const execute = section.getByRole('button', { name: 'Execute action' });

    await section.getByLabel('Target workspace ID').fill(ids.workspaceId);
    await section.getByLabel('Reason').fill('ticket REG-123 approved');
    await section.getByLabel('Confirmation code').fill('RECORD REGION EXCEPTION');
    await expect(execute).toBeDisabled();

    await section.getByLabel('Confirmation code').fill('RECORD REGION EXCEPTION AAAAAAAA');
    await expect(execute).toBeEnabled();
    await execute.click();

    await expect(
      page.getByText(`internal_admin.region.exception.recorded - recorded - ${ids.workspaceId}`),
    ).toBeVisible();
    expect(posts).toContainEqual(
      expect.objectContaining({
        confirmCode: 'RECORD REGION EXCEPTION AAAAAAAA',
        hasIdempotencyKey: true,
        path: `/workspaces/${ids.workspaceId}/admin/region/workspaces/${ids.workspaceId}/exceptions`,
        secondApproverPrincipalId: ids.secondApproverId,
        secondApproverRole: 'platform_admin',
      }),
    );
  });

  test('Region Center requires workspace-bound flag confirmation', async ({ page }) => {
    const posts: MutationPost[] = [];
    await captureMutation(page, posts);

    await page.goto('/');
    const section = page.locator('section#region-center');
    const execute = section.getByRole('button', { name: 'Execute action' });

    await section.getByLabel('Target workspace ID').fill(ids.workspaceId);
    await section.getByLabel('Reason').fill('ticket REG-456 approved');
    await section.getByLabel('Confirmation code').fill('FLAG RESIDENCY');
    await expect(execute).toBeDisabled();

    await section.getByLabel('Confirmation code').fill('FLAG RESIDENCY AAAAAAAA');
    await expect(execute).toBeEnabled();
    await execute.click();

    await expect(
      page.getByText(`internal_admin.region.residency.flagged - applied - ${ids.workspaceId}`),
    ).toBeVisible();
    expect(posts).toContainEqual(
      expect.objectContaining({
        confirmCode: 'FLAG RESIDENCY AAAAAAAA',
        hasIdempotencyKey: true,
        path: `/workspaces/${ids.workspaceId}/admin/region/workspaces/${ids.workspaceId}/residency-flag`,
        secondApproverPrincipalId: ids.secondApproverId,
        secondApproverRole: 'platform_admin',
      }),
    );
  });

  test('Risk Decision Center requires policy-bound block confirmation', async ({ page }) => {
    const posts: MutationPost[] = [];
    await captureMutation(page, posts);

    await page.goto('/');
    const section = page.locator('section#risk-decision-center');
    await section.getByRole('button', { name: 'Block policy' }).click();
    const execute = section.getByRole('button', { name: 'Execute action' });

    await section.getByLabel('Policy snapshot ID').fill(ids.riskPolicyId);
    await section.getByLabel('Reason').fill('ticket RISK-123 reviewed');
    await section.getByLabel('Confirmation code').fill('BLOCK RISK POLICY');
    await expect(execute).toBeDisabled();

    await section.getByLabel('Confirmation code').fill('BLOCK RISK POLICY 99999999');
    await expect(execute).toBeEnabled();
    await execute.click();

    await expect(
      page.getByText(`internal_admin.risk.policy.blocked - blocked - ${ids.riskPolicyId}`),
    ).toBeVisible();
    expect(posts).toContainEqual(
      expect.objectContaining({
        confirmCode: 'BLOCK RISK POLICY 99999999',
        hasIdempotencyKey: true,
        path: `/workspaces/${ids.workspaceId}/admin/risk/policies/${ids.riskPolicyId}/block`,
        secondApproverPrincipalId: ids.secondApproverId,
        secondApproverRole: 'platform_admin',
      }),
    );
  });

  test('Risk Decision Center requires signal-bound resolve confirmation', async ({ page }) => {
    const posts: MutationPost[] = [];
    await captureMutation(page, posts);

    await page.goto('/');
    const section = page.locator('section#risk-decision-center');
    const execute = section.getByRole('button', { name: 'Execute action' });

    await section.getByLabel('Risk signal ID').fill(ids.riskSignalId);
    await section.getByLabel('Reason').fill('ticket RISK-456 reviewed');
    await section.getByLabel('Confirmation code').fill('RESOLVE RISK SIGNAL');
    await expect(execute).toBeDisabled();

    await section.getByLabel('Confirmation code').fill('RESOLVE RISK SIGNAL 12121212');
    await expect(execute).toBeEnabled();
    await execute.click();

    await expect(
      page.getByText(`internal_admin.risk.signal.resolved - resolved - ${ids.riskSignalId}`),
    ).toBeVisible();
    expect(posts).toContainEqual(
      expect.objectContaining({
        confirmCode: 'RESOLVE RISK SIGNAL 12121212',
        hasIdempotencyKey: true,
        path: `/workspaces/${ids.workspaceId}/admin/risk/signals/${ids.riskSignalId}/resolve`,
        secondApproverPrincipalId: ids.secondApproverId,
        secondApproverRole: 'platform_admin',
      }),
    );
  });
});
