import { expect, test } from '@playwright/test';
import * as ids from './action-confirmation.fixtures';
import {
  captureMutation,
  installCredentials,
  installMockApi,
  openDetailFromGlobalSearch,
  type MutationPost,
} from './action-confirmation.mock-api';

test.describe('backoffice-service product and operations action flows @mocked', () => {
  test.beforeEach(async ({ page }) => {
    await installCredentials(page);
    await installMockApi(page);
  });

  test('Entitlements Center requires feature-bound grant confirmation', async ({ page }) => {
    const posts: MutationPost[] = [];
    await captureMutation(page, posts);

    await page.goto('/');
    const section = page.locator('section#entitlements-center');
    const execute = section.getByRole('button', { name: 'Execute action' });

    await section.getByLabel('Reason').fill('ticket ENT-123 approved');
    await section.getByLabel('Confirmation code').fill('GRANT FEATURE');
    await expect(execute).toBeDisabled();

    await section.getByLabel('Confirmation code').fill('GRANT FEATURE ADVANCED');
    await expect(execute).toBeEnabled();
    await execute.click();

    await expect(
      page.getByText('internal_admin.entitlements.feature.granted - queued - advanced_search'),
    ).toBeVisible();
    expect(posts).toContainEqual(
      expect.objectContaining({
        confirmCode: 'GRANT FEATURE ADVANCED',
        hasIdempotencyKey: true,
        path: `/workspaces/${ids.workspaceId}/admin/entitlements/grants`,
        secondApproverPrincipalId: ids.secondApproverId,
        secondApproverRole: 'platform_admin',
      }),
    );
  });

  test('Usage Center requires meter-bound correction confirmation', async ({ page }) => {
    const posts: MutationPost[] = [];
    await captureMutation(page, posts);

    await page.goto('/');
    const section = page.locator('section#usage-center');
    const execute = section.getByRole('button', { name: 'Execute action' });

    await section.getByLabel('Reason').fill('ticket USG-123 approved');
    await section.getByLabel('Confirmation code').fill('CORRECT USAGE');
    await expect(execute).toBeDisabled();

    await section.getByLabel('Confirmation code').fill('CORRECT USAGE API_CALL');
    await expect(execute).toBeEnabled();
    await execute.click();

    await expect(
      page.getByText('internal_admin.usage.correction.created - applied - api_call'),
    ).toBeVisible();
    expect(posts).toContainEqual(
      expect.objectContaining({
        confirmCode: 'CORRECT USAGE API_CALL',
        hasIdempotencyKey: true,
        path: `/workspaces/${ids.workspaceId}/admin/usage/corrections`,
        secondApproverPrincipalId: ids.secondApproverId,
        secondApproverRole: 'platform_admin',
      }),
    );
  });

  test('Operations Center requires object-bound provider replay confirmation', async ({ page }) => {
    const posts: MutationPost[] = [];
    await captureMutation(page, posts);

    await page.goto('/');
    const section = page.locator('section#operations-center');
    const execute = section.getByRole('button', { name: 'Execute action' });

    await section.getByLabel('Provider event ID').fill(ids.providerEventId);
    await section.getByLabel('Reason').fill('ticket OPS-123 approved');
    await section.getByLabel('Confirmation code').fill('REPLAY PROVIDER EVENT');
    await expect(execute).toBeDisabled();

    await section.getByLabel('Confirmation code').fill('REPLAY PROVIDER EVENT 66666666');
    await expect(execute).toBeEnabled();
    await execute.click();

    await expect(
      page.getByText(
        `internal_admin.operations.provider_event.replayed - queued - ${ids.providerEventId}`,
      ),
    ).toBeVisible();
    expect(posts).toContainEqual(
      expect.objectContaining({
        confirmCode: 'REPLAY PROVIDER EVENT 66666666',
        hasIdempotencyKey: true,
        path: `/workspaces/${ids.workspaceId}/admin/operations/provider-events/${ids.providerEventId}/replay`,
        secondApproverPrincipalId: ids.secondApproverId,
        secondApproverRole: 'platform_admin',
      }),
    );
  });

  test('Operations Center requires incident-bound state confirmation', async ({ page }) => {
    const posts: MutationPost[] = [];
    await captureMutation(page, posts);

    await page.goto('/');
    const section = page.locator('section#operations-center');
    await section.getByRole('button', { name: 'Incident' }).click();
    const execute = section.getByRole('button', { name: 'Execute action' });

    await section.getByLabel('Incident ID').fill(ids.incidentId);
    await section.getByLabel('Reason').fill('ticket OPS-456 approved');
    await section.getByLabel('Confirmation code').fill('UPDATE INCIDENT');
    await expect(execute).toBeDisabled();

    await section.getByLabel('Confirmation code').fill('UPDATE INCIDENT 67676767');
    await expect(execute).toBeEnabled();
    await execute.click();

    await expect(
      page.getByText(
        `internal_admin.operations.incident.state_updated - mitigating - ${ids.incidentId}`,
      ),
    ).toBeVisible();
    expect(posts).toContainEqual(
      expect.objectContaining({
        confirmCode: 'UPDATE INCIDENT 67676767',
        hasIdempotencyKey: true,
        path: `/workspaces/${ids.workspaceId}/admin/operations/incidents/${ids.incidentId}/state`,
        secondApproverPrincipalId: ids.secondApproverId,
        secondApproverRole: 'platform_admin',
      }),
    );
  });

  test('Operations Center requires workspace-bound maintenance confirmation', async ({ page }) => {
    const posts: MutationPost[] = [];
    await captureMutation(page, posts);

    await page.goto('/');
    const section = page.locator('section#operations-center');
    await section.getByRole('button', { name: 'Maintenance' }).click();
    const execute = section.getByRole('button', { name: 'Execute action' });

    await section.getByLabel('Maintenance title').fill('Database maintenance');
    await section.getByLabel('Start time').fill('2026-07-01T02:00:00Z');
    await section.getByLabel('End time').fill('2026-07-01T04:00:00Z');
    await section.getByLabel('Reason').fill('ticket OPS-789 approved');
    await section.getByLabel('Confirmation code').fill('SCHEDULE MAINTENANCE');
    await expect(execute).toBeDisabled();

    await section.getByLabel('Confirmation code').fill('SCHEDULE MAINTENANCE AAAAAAAA');
    await expect(execute).toBeEnabled();
    await execute.click();

    await expect(
      page.getByText(
        `internal_admin.operations.maintenance_window.scheduled - scheduled - ${ids.workspaceId}`,
      ),
    ).toBeVisible();
    expect(posts).toContainEqual(
      expect.objectContaining({
        confirmCode: 'SCHEDULE MAINTENANCE AAAAAAAA',
        hasIdempotencyKey: true,
        path: `/workspaces/${ids.workspaceId}/admin/operations/maintenance-windows`,
        secondApproverPrincipalId: ids.secondApproverId,
        secondApproverRole: 'platform_admin',
      }),
    );
  });

  test('Developer Center requires client-bound revoke confirmation', async ({ page }) => {
    const posts: MutationPost[] = [];
    await captureMutation(page, posts);

    await page.goto('/');
    const section = page.locator('section#developer-center');
    const execute = section.getByRole('button', { name: 'Execute action' });

    await section.getByLabel('Client ID').fill(ids.developerClientId);
    await section.getByLabel('Reason').fill('ticket DEV-123 approved');
    await section.getByLabel('Confirmation code').fill('REVOKE CLIENT');
    await expect(execute).toBeDisabled();

    await section.getByLabel('Confirmation code').fill('REVOKE CLIENT 77777777');
    await expect(execute).toBeEnabled();
    await execute.click();

    await expect(
      page.getByText(
        `internal_admin.developer.client.revoked - revoked - ${ids.developerClientId}`,
      ),
    ).toBeVisible();
    expect(posts).toContainEqual(
      expect.objectContaining({
        confirmCode: 'REVOKE CLIENT 77777777',
        hasIdempotencyKey: true,
        path: `/workspaces/${ids.workspaceId}/admin/developer/clients/${ids.developerClientId}/revoke`,
        secondApproverPrincipalId: ids.secondApproverId,
        secondApproverRole: 'platform_admin',
      }),
    );
  });

  test('Communications Center requires message-bound replay confirmation', async ({ page }) => {
    const posts: MutationPost[] = [];
    await captureMutation(page, posts);

    await page.goto('/');
    const section = page.locator('section#communications-center');
    const execute = section.getByRole('button', { name: 'Execute action' });

    await section.getByLabel('Email message ID').fill(ids.emailMessageId);
    await section.getByLabel('Reason').fill('ticket COMMS-123 approved');
    await section.getByLabel('Confirmation code').fill('REPLAY EMAIL');
    await expect(execute).toBeDisabled();

    await section.getByLabel('Confirmation code').fill('REPLAY EMAIL 88888888');
    await expect(execute).toBeEnabled();
    await execute.click();

    await expect(
      page.getByText(
        `internal_admin.communications.email.replayed - queued - ${ids.emailMessageId}`,
      ),
    ).toBeVisible();
    expect(posts).toContainEqual(
      expect.objectContaining({
        confirmCode: 'REPLAY EMAIL 88888888',
        hasIdempotencyKey: true,
        path: `/workspaces/${ids.workspaceId}/admin/communications/emails/${ids.emailMessageId}/replay`,
        secondApproverPrincipalId: ids.secondApproverId,
        secondApproverRole: 'platform_admin',
      }),
    );
  });
});
