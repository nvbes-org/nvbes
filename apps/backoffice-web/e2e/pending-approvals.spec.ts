import { expect, test } from '@playwright/test';
import * as ids from './action-confirmation.fixtures';
import {
  captureMutation,
  installCredentials,
  installMockApi,
  type MutationPost,
} from './action-confirmation.mock-api';

test.describe('backoffice-service pending approvals @mocked', () => {
  test.beforeEach(async ({ page }) => {
    await installCredentials(page);
    await installMockApi(page);
  });

  test('renders dual-control work and opens linked target details', async ({ page }) => {
    await page.goto('/#pending-approvals');
    const section = page.locator('section#pending-approvals');

    await expect(section.getByText('Pending approvals')).toBeVisible();
    await expect(section.getByText('Approve marketplace app', { exact: true })).toBeVisible();
    await expect(section.getByText('Review recovery request')).toBeVisible();
    await expect(section.getByText('security_admin')).toBeVisible();
    await expect(section.getByText('Identity Governance - CANCEL RECOVERY')).toBeVisible();

    await section
      .locator('tr')
      .filter({ hasText: 'Review recovery request' })
      .getByRole('button', { name: 'User' })
      .click();

    await expect(page).toHaveURL(/#user-detail/u);
    await expect(
      page.locator('section#user-detail').getByText('customer-user@example.test'),
    ).toBeVisible();
  });

  test('executes marketplace approval with strong confirmation and dual control', async ({
    page,
  }) => {
    const posts: MutationPost[] = [];
    await captureMutation(page, posts);
    await page.goto('/#pending-approvals');
    const row = page.locator('tr').filter({ hasText: 'Approve marketplace app' });

    await row.getByRole('button', { name: 'Approve' }).click();
    await row.getByPlaceholder('APPROVE MARKETPLACE APP 77777777').fill('APPROVE MARKETPLACE APP');
    await row.getByPlaceholder('ticket APPROVAL-123 reviewed').fill('ticket APPROVAL-123 reviewed');
    await expect(row.getByRole('button', { name: 'Execute approval' })).toBeDisabled();

    await row
      .getByPlaceholder('APPROVE MARKETPLACE APP 77777777')
      .fill('APPROVE MARKETPLACE APP 77777777');
    await expect(row.getByRole('button', { name: 'Execute approval' })).toBeEnabled();
    await row.getByRole('button', { name: 'Execute approval' }).click();

    expect(posts).toContainEqual(
      expect.objectContaining({
        confirmCode: 'APPROVE MARKETPLACE APP 77777777',
        hasIdempotencyKey: true,
        path: `/workspaces/${ids.workspaceId}/admin/developer/marketplace-apps/${ids.developerClientId}/approve`,
        secondApproverPrincipalId: ids.secondApproverId,
        secondApproverRole: 'platform_admin',
      }),
    );
  });
});
