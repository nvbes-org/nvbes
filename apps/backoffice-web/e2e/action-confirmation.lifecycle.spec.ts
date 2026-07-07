import { expect, test } from '@playwright/test';
import * as ids from './action-confirmation.fixtures';
import {
  captureMutation,
  installCredentials,
  installMockApi,
  openDetailFromGlobalSearch,
  type MutationPost,
} from './action-confirmation.mock-api';

test.describe('backoffice-service customer lifecycle action flows @mocked', () => {
  test.beforeEach(async ({ page }) => {
    await installCredentials(page);
    await installMockApi(page);
  });

  test('Tenant detail requires confirmation before customer lifecycle suspension', async ({
    page,
  }) => {
    const posts: MutationPost[] = [];
    await captureMutation(page, posts);

    await openDetailFromGlobalSearch(page, 'acme', 'Open tenant');
    const section = page.locator('section#tenant-detail');

    await section.getByRole('button', { name: 'Suspendre' }).click();
    await section.getByPlaceholder('Motif audit').fill('ticket CUST-tenant approved');
    await section.getByPlaceholder('SUSPEND TENANT BBBBBBBB').fill('SUSPEND TENANT');
    await expect(section.getByRole('button', { name: 'Confirmer' })).toBeDisabled();

    await section.getByPlaceholder('SUSPEND TENANT BBBBBBBB').fill('SUSPEND TENANT BBBBBBBB');
    await expect(section.getByRole('button', { name: 'Confirmer' })).toBeEnabled();
    await section.getByRole('button', { name: 'Confirmer' }).click();

    await expect(
      page.getByText('internal_admin.tenant.suspend: active vers suspended'),
    ).toBeVisible();
    expect(posts).toContainEqual(
      expect.objectContaining({
        confirmCode: 'SUSPEND TENANT BBBBBBBB',
        hasIdempotencyKey: true,
        path: `/admin/tenants/${ids.tenantId}/suspend`,
        secondApproverPrincipalId: ids.secondApproverId,
        secondApproverRole: 'platform_admin',
      }),
    );
  });

  test('Workspace detail requires confirmation before customer lifecycle suspension', async ({
    page,
  }) => {
    const posts: MutationPost[] = [];
    await captureMutation(page, posts);

    await openDetailFromGlobalSearch(page, 'workspace', 'Open workspace');
    const section = page.locator('section#workspace-detail');

    await section.getByRole('button', { name: 'Suspendre' }).click();
    await section.getByPlaceholder('Motif audit').fill('ticket CUST-workspace approved');
    await section.getByPlaceholder('SUSPEND WORKSPACE AAAAAAAA').fill('SUSPEND WORKSPACE');
    await expect(section.getByRole('button', { name: 'Confirmer' })).toBeDisabled();

    await section.getByPlaceholder('SUSPEND WORKSPACE AAAAAAAA').fill('SUSPEND WORKSPACE AAAAAAAA');
    await expect(section.getByRole('button', { name: 'Confirmer' })).toBeEnabled();
    await section.getByRole('button', { name: 'Confirmer' }).click();

    await expect(
      page.getByText('internal_admin.workspace.suspend: active vers suspended'),
    ).toBeVisible();
    expect(posts).toContainEqual(
      expect.objectContaining({
        confirmCode: 'SUSPEND WORKSPACE AAAAAAAA',
        hasIdempotencyKey: true,
        path: `/admin/workspaces/${ids.workspaceId}/suspend`,
        secondApproverPrincipalId: ids.secondApproverId,
        secondApproverRole: 'platform_admin',
      }),
    );
  });

  test('User detail requires confirmation before customer lifecycle suspension', async ({
    page,
  }) => {
    const posts: MutationPost[] = [];
    await captureMutation(page, posts);

    await openDetailFromGlobalSearch(page, 'user', 'Open user');
    const section = page.locator('section#user-detail');

    await section.getByRole('button', { name: 'Suspendre' }).click();
    await section.getByPlaceholder('Motif audit').fill('ticket CUST-user approved');
    await section.getByPlaceholder('SUSPEND USER 11111111').fill('SUSPEND USER');
    await expect(section.getByRole('button', { name: 'Confirmer' })).toBeDisabled();

    await section.getByPlaceholder('SUSPEND USER 11111111').fill('SUSPEND USER 11111111');
    await expect(section.getByRole('button', { name: 'Confirmer' })).toBeEnabled();
    await section.getByRole('button', { name: 'Confirmer' }).click();

    await expect(
      page.getByText('internal_admin.user.suspend: active vers suspended'),
    ).toBeVisible();
    expect(posts).toContainEqual(
      expect.objectContaining({
        confirmCode: 'SUSPEND USER 11111111',
        hasIdempotencyKey: true,
        path: `/admin/users/${ids.principalId}/suspend`,
        secondApproverPrincipalId: ids.secondApproverId,
        secondApproverRole: 'platform_admin',
      }),
    );
  });
});
