import { expect, test } from '@playwright/test';
import * as ids from './action-confirmation.fixtures';
import {
  installCredentials,
  installMockApi,
  rejectMutations,
  type MutationPost,
} from './action-confirmation.mock-api';

test.describe('backoffice-service RBAC mutation failures @mocked', () => {
  test.beforeEach(async ({ page }) => {
    await installMockApi(page);
  });

  test('Developer Center surfaces forbidden role failures without hiding audit headers', async ({
    page,
  }) => {
    const posts: MutationPost[] = [];
    await installCredentials(page, 'finance_admin');
    await rejectMutations(page, posts, 'Forbidden: developer_admin permission required');

    await page.goto('/');
    const section = page.locator('section#developer-center');
    const execute = section.getByRole('button', { name: 'Execute action' });

    await section.getByLabel('Client ID').fill(ids.developerClientId);
    await section.getByLabel('Reason').fill('ticket DEV-403 approved');
    await section.getByLabel('Confirmation code').fill('REVOKE CLIENT 77777777');
    await expect(execute).toBeEnabled();
    await execute.click();

    await expect(section.getByText('Forbidden: developer_admin permission required')).toBeVisible();
    expect(posts).toContainEqual(
      expect.objectContaining({
        backofficeRole: 'finance_admin',
        confirmCode: 'REVOKE CLIENT 77777777',
        hasIdempotencyKey: true,
        path: `/workspaces/${ids.workspaceId}/admin/developer/clients/${ids.developerClientId}/revoke`,
        secondApproverPrincipalId: ids.secondApproverId,
        secondApproverRole: 'platform_admin',
      }),
    );
  });

  test('Entitlements Center surfaces forbidden role failures for finance operators', async ({
    page,
  }) => {
    const posts: MutationPost[] = [];
    await installCredentials(page, 'finance_admin');
    await rejectMutations(page, posts, 'Forbidden: product_admin permission required');

    await page.goto('/');
    const section = page.locator('section#entitlements-center');

    await section.getByLabel('Reason').fill('ticket ENT-403 approved');
    await section.getByLabel('Confirmation code').fill('GRANT FEATURE ADVANCED');
    await section.getByRole('button', { name: 'Execute action' }).click();

    await expect(section.getByText('Forbidden: product_admin permission required')).toBeVisible();
    expect(posts).toContainEqual(
      expect.objectContaining({
        backofficeRole: 'finance_admin',
        confirmCode: 'GRANT FEATURE ADVANCED',
        hasIdempotencyKey: true,
        path: `/workspaces/${ids.workspaceId}/admin/entitlements/grants`,
        secondApproverPrincipalId: ids.secondApproverId,
        secondApproverRole: 'platform_admin',
      }),
    );
  });

  test('Operations Center surfaces forbidden role failures for finance operators', async ({
    page,
  }) => {
    const posts: MutationPost[] = [];
    await installCredentials(page, 'finance_admin');
    await rejectMutations(page, posts, 'Forbidden: operations_admin permission required');

    await page.goto('/');
    const section = page.locator('section#operations-center');

    await section.getByLabel('Provider event ID').fill(ids.providerEventId);
    await section.getByLabel('Reason').fill('ticket OPS-403 approved');
    await section.getByLabel('Confirmation code').fill('REPLAY PROVIDER EVENT 66666666');
    await section.getByRole('button', { name: 'Execute action' }).click();

    await expect(
      section.getByText('Forbidden: operations_admin permission required'),
    ).toBeVisible();
    expect(posts).toContainEqual(
      expect.objectContaining({
        backofficeRole: 'finance_admin',
        confirmCode: 'REPLAY PROVIDER EVENT 66666666',
        hasIdempotencyKey: true,
        path: `/workspaces/${ids.workspaceId}/admin/operations/provider-events/${ids.providerEventId}/replay`,
        secondApproverPrincipalId: ids.secondApproverId,
        secondApproverRole: 'platform_admin',
      }),
    );
  });

  test('Compliance Center surfaces forbidden role failures for finance operators', async ({
    page,
  }) => {
    const posts: MutationPost[] = [];
    await installCredentials(page, 'finance_admin');
    await rejectMutations(page, posts, 'Forbidden: compliance_admin permission required');

    await page.goto('/');
    const section = page.locator('section#compliance-center');

    await section.getByLabel('Consent ID').fill(ids.consentId);
    await section.getByLabel('Reason').fill('ticket GDPR-403 approved');
    await section.getByLabel('Confirmation code').fill('REVOKE CONSENT ABABABAB');
    await section.getByRole('button', { name: 'Execute action' }).click();

    await expect(
      section.getByText('Forbidden: compliance_admin permission required'),
    ).toBeVisible();
    expect(posts).toContainEqual(
      expect.objectContaining({
        backofficeRole: 'finance_admin',
        confirmCode: 'REVOKE CONSENT ABABABAB',
        hasIdempotencyKey: true,
        path: `/workspaces/${ids.workspaceId}/admin/compliance/consents/${ids.consentId}/revoke`,
        secondApproverPrincipalId: ids.secondApproverId,
        secondApproverRole: 'platform_admin',
      }),
    );
  });

  test('Revenue Center surfaces forbidden role failures for developer operators', async ({
    page,
  }) => {
    const posts: MutationPost[] = [];
    await installCredentials(page, 'developer_admin');
    await rejectMutations(page, posts, 'Forbidden: finance_admin permission required');

    await page.goto('/');
    const section = page.locator('section#revenue-center');

    await section.getByLabel('Invoice ID').fill(ids.invoiceId);
    await section.getByLabel('Reason').fill('ticket REV-403 approved');
    await section.getByLabel('Confirmation code').fill('HOLD INVOICE 44444444');
    await section.getByRole('button', { name: 'Execute action' }).click();

    await expect(section.getByText('Forbidden: finance_admin permission required')).toBeVisible();
    expect(posts).toContainEqual(
      expect.objectContaining({
        backofficeRole: 'developer_admin',
        confirmCode: 'HOLD INVOICE 44444444',
        hasIdempotencyKey: true,
        path: `/workspaces/${ids.workspaceId}/admin/revenue/invoices/${ids.invoiceId}/hold`,
      }),
    );
  });
});
