import { expect, test } from '@playwright/test';
import {
  installCredentials,
  installMockApi,
  openDetailFromGlobalSearch,
} from './action-confirmation.mock-api';

test.describe('backoffice-service entity detail tabs @mocked', () => {
  test.beforeEach(async ({ page }) => {
    await installCredentials(page);
    await installMockApi(page);
  });

  test('tenant detail exposes audit billing security usage and access tabs', async ({ page }) => {
    await openDetailFromGlobalSearch(page, 'acme', 'Open tenant');
    const section = page.locator('section#tenant-detail');
    const tabs = section.getByTestId('entity-detail-tabs');

    await expect(tabs.getByRole('tab', { name: 'Audit' })).toBeVisible();
    await tabs.getByRole('tab', { name: 'Billing' }).click();
    await expect(tabs.getByText('Open invoices')).toBeVisible();
    await expect(tabs.getByRole('button', { name: 'Platform' })).toBeVisible();

    await tabs.getByRole('tab', { name: 'Security' }).click();
    await expect(tabs.getByText('Posture securite')).toBeVisible();

    await tabs.getByRole('tab', { name: 'Usage' }).click();
    await expect(tabs.getByText('Tenant scope')).toBeVisible();

    await tabs.getByRole('tab', { name: 'Access' }).click();
    await expect(tabs.getByRole('button', { name: 'Governance' })).toBeVisible();
  });

  test('workspace detail tabs keep operators close to billing and access centers', async ({
    page,
  }) => {
    await openDetailFromGlobalSearch(page, 'workspace', 'Open workspace');
    const section = page.locator('section#workspace-detail');
    const tabs = section.getByTestId('entity-detail-tabs');

    await tabs.getByRole('tab', { name: 'Billing' }).click();
    await expect(tabs.getByText('Active subs')).toBeVisible();
    await tabs.getByRole('button', { name: 'Platform' }).click();
    await expect(page).toHaveURL(/#billing-platform-center/u);

    await openDetailFromGlobalSearch(page, 'workspace', 'Open workspace');
    await tabs.getByRole('tab', { name: 'Access' }).click();
    await expect(tabs.getByText('Active access')).toBeVisible();
  });

  test('user detail tabs surface security risk and access state', async ({ page }) => {
    await openDetailFromGlobalSearch(page, 'user', 'Open user');
    const section = page.locator('section#user-detail');
    const tabs = section.getByTestId('entity-detail-tabs');

    await tabs.getByRole('tab', { name: 'Security' }).click();
    await expect(tabs.getByText('MFA active')).toBeVisible();
    await expect(tabs.getByText('Risk 24h')).toBeVisible();

    await tabs.getByRole('tab', { name: 'Access' }).click();
    await expect(tabs.getByText('Principal', { exact: true })).toBeVisible();
    await expect(tabs.getByRole('button', { name: 'Governance' })).toBeVisible();
  });
});
