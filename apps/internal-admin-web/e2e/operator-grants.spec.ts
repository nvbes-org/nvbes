import { expect, test } from '@playwright/test';
import * as ids from './action-confirmation.fixtures';
import {
  captureMutation,
  installCredentials,
  installMockApi,
  type MutationPost,
} from './action-confirmation.mock-api';

test.describe('internal-admin operator grants @mocked', () => {
  test.beforeEach(async ({ page }) => {
    await installCredentials(page);
    await installMockApi(page);
  });

  test('surfaces back-office operator grants and role distribution', async ({ page }) => {
    await page.goto('/#identity-governance-center');
    const section = page.locator('section#identity-governance-center');
    const grants = section.getByTestId('operator-grants-panel');

    await expect(
      grants.getByRole('heading', { name: 'Back-office operator grants' }),
    ).toBeVisible();
    await expect(grants.getByText('platform_admin: 1')).toBeVisible();
    await expect(grants.getByText('security_admin: 1')).toBeVisible();
    await expect(grants.getByText('operator@example.test', { exact: true })).toBeVisible();
    await expect(grants.getByText('former-operator@example.test', { exact: true })).toBeVisible();
    await expect(grants.getByText('revoked', { exact: true })).toBeVisible();
  });

  test('grants operator roles with object-bound confirmation and idempotency', async ({ page }) => {
    const posts: MutationPost[] = [];
    await captureMutation(page, posts);
    await page.goto('/#identity-governance-center');
    const grants = page
      .locator('section#identity-governance-center')
      .getByTestId('operator-grants-panel');

    await grants.getByPlaceholder('Principal UUID').fill(ids.principalId);
    await grants.getByRole('button', { name: 'Grant role' }).click();
    await grants.getByPlaceholder('Motif audit').fill('ticket IAM-999 approved');
    await grants.getByPlaceholder('GRANT OPERATOR VIEWER 11111111').fill('GRANT OPERATOR VIEWER');
    await expect(grants.getByRole('button', { name: 'Confirmer' })).toBeDisabled();

    await grants
      .getByPlaceholder('GRANT OPERATOR VIEWER 11111111')
      .fill('GRANT OPERATOR VIEWER 11111111');
    await expect(grants.getByRole('button', { name: 'Confirmer' })).toBeEnabled();
    await grants.getByRole('button', { name: 'Confirmer' }).click();

    await expect(
      grants.getByText('internal_admin.identity_governance.operator_grant.granted'),
    ).toBeVisible();
    expect(posts).toContainEqual(
      expect.objectContaining({
        confirmCode: 'GRANT OPERATOR VIEWER 11111111',
        hasIdempotencyKey: true,
        path: `/admin/identity-governance-center/operator-grants/${ids.principalId}/viewer/grant`,
        secondApproverPrincipalId: ids.secondApproverId,
        secondApproverRole: 'platform_admin',
      }),
    );
  });

  test('revokes active operator grants with object-bound confirmation', async ({ page }) => {
    const posts: MutationPost[] = [];
    await captureMutation(page, posts);
    await page.goto('/#identity-governance-center');
    const grants = page
      .locator('section#identity-governance-center')
      .getByTestId('operator-grants-panel');
    const row = grants.locator('article').filter({ hasText: ids.actorId });

    await row.getByRole('button', { name: 'Revoke' }).click();
    await grants.getByPlaceholder('Motif audit').fill('ticket IAM-1000 approved');
    await grants
      .getByPlaceholder('REVOKE OPERATOR PLATFORM_ADMIN CCCCCCCC')
      .fill('REVOKE OPERATOR PLATFORM_ADMIN CCCCCCCC');
    await grants.getByRole('button', { name: 'Confirmer' }).click();

    await expect(
      grants.getByText('internal_admin.identity_governance.operator_grant.revoked'),
    ).toBeVisible();
    expect(posts).toContainEqual(
      expect.objectContaining({
        confirmCode: 'REVOKE OPERATOR PLATFORM_ADMIN CCCCCCCC',
        hasIdempotencyKey: true,
        path: `/admin/identity-governance-center/operator-grants/${ids.actorId}/platform_admin/revoke`,
      }),
    );
  });
});
