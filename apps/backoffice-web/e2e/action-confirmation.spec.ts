import { expect, test } from '@playwright/test';
import * as ids from './action-confirmation.fixtures';
import {
  captureMutation,
  installCredentials,
  installMockApi,
  type MutationPost,
} from './action-confirmation.mock-api';

test.describe('backoffice-service actionable flows @mocked', () => {
  test.beforeEach(async ({ page }) => {
    await installCredentials(page);
    await installMockApi(page);
  });

  test('Access Center requires strong confirmation and sends idempotent mutation', async ({
    page,
  }) => {
    const posts: MutationPost[] = [];
    await captureMutation(page, posts);

    await page.goto('/');
    const article = page.locator('article').filter({ hasText: 'access-admin@example.test' });
    await article.getByRole('button', { name: 'Suspend access' }).click();

    await article.getByPlaceholder('Motif audit').fill('ticket IAM-123 approved');
    await article.getByPlaceholder('SUSPEND ACCESS 11111111').fill('SUSPEND ACCESS');
    await expect(article.getByRole('button', { name: 'Confirmer' })).toBeDisabled();

    await article.getByPlaceholder('SUSPEND ACCESS 11111111').fill('SUSPEND ACCESS 11111111');
    await expect(article.getByRole('button', { name: 'Confirmer' })).toBeEnabled();
    await article.getByRole('button', { name: 'Confirmer' }).click();

    await expect(
      article.getByText('internal_admin.access.workspace_membership.suspended'),
    ).toBeVisible();
    expect(posts).toContainEqual(
      expect.objectContaining({
        confirmCode: 'SUSPEND ACCESS 11111111',
        hasIdempotencyKey: true,
        path: `/admin/access-center/workspace-memberships/${ids.workspaceId}/${ids.principalId}/suspend`,
        secondApproverPrincipalId: ids.secondApproverId,
        secondApproverRole: 'platform_admin',
      }),
    );
  });

  test('Security Center requires object-bound MFA confirmation', async ({ page }) => {
    const posts: MutationPost[] = [];
    await captureMutation(page, posts);

    await page.goto('/');
    const article = page.locator('article').filter({ hasText: 'security@example.test' }).first();
    await article.getByRole('button', { name: 'Revoquer' }).click();

    await article.getByPlaceholder('Motif audit').fill('ticket SEC-789 approved');
    await article.getByPlaceholder('REVOKE MFA 22222222').fill('REVOKE MFA');
    await expect(article.getByRole('button', { name: 'Confirmer' })).toBeDisabled();

    await article.getByPlaceholder('REVOKE MFA 22222222').fill('REVOKE MFA 22222222');
    await expect(article.getByRole('button', { name: 'Confirmer' })).toBeEnabled();
    await article.getByRole('button', { name: 'Confirmer' }).click();

    await expect(
      page.getByText(`internal_admin.security.mfa_factor.revoked: ${ids.mfaFactorId}`),
    ).toBeVisible();
    expect(posts).toContainEqual(
      expect.objectContaining({
        confirmCode: 'REVOKE MFA 22222222',
        hasIdempotencyKey: true,
        path: `/admin/security-center/mfa-factors/${ids.mfaFactorId}/revoke`,
        secondApproverPrincipalId: ids.secondApproverId,
        secondApproverRole: 'platform_admin',
      }),
    );
  });

  test('Identity Governance requires object-bound break-glass confirmation', async ({ page }) => {
    const posts: MutationPost[] = [];
    await captureMutation(page, posts);

    await page.goto('/');
    const article = page.locator('article').filter({ hasText: 'break glass emergency' });
    await article.getByRole('button', { name: 'Revoquer' }).click();

    await article.getByPlaceholder('Motif audit').fill('ticket GOV-456 approved');
    await article.getByPlaceholder('REVOKE BREAK GLASS 11111111').fill('REVOKE BREAK GLASS');
    await expect(article.getByRole('button', { name: 'Confirmer' })).toBeDisabled();

    await article
      .getByPlaceholder('REVOKE BREAK GLASS 11111111')
      .fill('REVOKE BREAK GLASS 11111111');
    await expect(article.getByRole('button', { name: 'Confirmer' })).toBeEnabled();
    await article.getByRole('button', { name: 'Confirmer' }).click();

    await expect(
      page.getByText(`internal_admin.identity_governance.break_glass.revoked: ${ids.principalId}`),
    ).toBeVisible();
    expect(posts).toContainEqual(
      expect.objectContaining({
        confirmCode: 'REVOKE BREAK GLASS 11111111',
        hasIdempotencyKey: true,
        path: `/admin/identity-governance-center/break-glass/${ids.tenantId}/${ids.principalId}/revoke`,
        secondApproverPrincipalId: ids.secondApproverId,
        secondApproverRole: 'platform_admin',
      }),
    );
  });

  test('Revenue Center requires object-bound invoice confirmation', async ({ page }) => {
    const posts: MutationPost[] = [];
    await captureMutation(page, posts);

    await page.goto('/');
    const section = page.locator('section#revenue-center');
    const execute = section.getByRole('button', { name: 'Execute action' });

    await section.getByLabel('Invoice ID').fill(ids.invoiceId);
    await section.getByLabel('Reason').fill('ticket REV-123 approved');
    await section.getByLabel('Confirmation code').fill('HOLD INVOICE');
    await expect(execute).toBeDisabled();

    await section.getByLabel('Confirmation code').fill('HOLD INVOICE 44444444');
    await expect(execute).toBeEnabled();
    await execute.click();

    await expect(
      page.getByText(`internal_admin.revenue.invoice.held - held - ${ids.invoiceId}`),
    ).toBeVisible();
    expect(posts).toContainEqual(
      expect.objectContaining({
        confirmCode: 'HOLD INVOICE 44444444',
        hasIdempotencyKey: true,
        path: `/workspaces/${ids.workspaceId}/admin/revenue/invoices/${ids.invoiceId}/hold`,
      }),
    );
  });

  test('Billing Platform requires object-bound routing confirmation', async ({ page }) => {
    const posts: MutationPost[] = [];
    await captureMutation(page, posts);

    await page.goto('/');
    const section = page.locator('section#billing-platform-center');
    await section.getByRole('button', { name: 'Disable routing' }).click();
    const execute = section.getByRole('button', { name: 'Execute action' });

    await section.getByLabel('Routing rule ID').fill(ids.routingRuleId);
    await section.getByLabel('Reason').fill('ticket BPL-123 approved');
    await section.getByLabel('Confirmation code').fill('DISABLE ROUTING RULE');
    await expect(execute).toBeDisabled();

    await section.getByLabel('Confirmation code').fill('DISABLE ROUTING RULE 55555555');
    await expect(execute).toBeEnabled();
    await execute.click();

    await expect(
      page.getByText(
        `internal_admin.billing_platform.routing_rule.disabled - disabled - ${ids.routingRuleId}`,
      ),
    ).toBeVisible();
    expect(posts).toContainEqual(
      expect.objectContaining({
        confirmCode: 'DISABLE ROUTING RULE 55555555',
        hasIdempotencyKey: true,
        path: `/workspaces/${ids.workspaceId}/admin/billing-platform/routing-rules/${ids.routingRuleId}/disable`,
      }),
    );
  });

  test('Billing Admin mutations require object-bound confirmation and dual control', async ({
    page,
  }) => {
    const posts: MutationPost[] = [];
    await captureMutation(page, posts);

    await page.goto('/#mutations');
    const section = page.locator('section#mutations');
    const execute = section.getByRole('button', { name: 'Executer' });

    await section.locator('input[name="invoice_id"]').fill(ids.invoiceId);
    await section.locator('input[name="amount_minor"]').fill('1200');
    await section.locator('textarea[name="reason"]').fill('ticket BILL-123 approved');
    await section.getByPlaceholder('Recopier le code de confirmation').fill('CREATE CREDIT NOTE');
    await expect(execute).toBeDisabled();

    await section
      .getByPlaceholder('Recopier le code de confirmation')
      .fill('CREATE CREDIT NOTE 44444444');
    await expect(execute).toBeEnabled();
    await execute.click();

    expect(posts).toContainEqual(
      expect.objectContaining({
        confirmCode: 'CREATE CREDIT NOTE 44444444',
        hasIdempotencyKey: true,
        path: `/workspaces/${ids.workspaceId}/billing/admin/credit-notes`,
        secondApproverPrincipalId: ids.secondApproverId,
        secondApproverRole: 'platform_admin',
      }),
    );
  });

  test('Billing runbooks require runbook-bound confirmation and dual control', async ({ page }) => {
    const posts: MutationPost[] = [];
    await captureMutation(page, posts);

    await page.goto('/#runbooks');
    const article = page.locator('article').filter({ hasText: 'PSP outage' });
    await article.getByRole('button', { name: 'Executer' }).click();

    await article
      .getByPlaceholder('Motif audit, incident, ticket, approbation...')
      .fill('incident BILL-999 approved');
    await article.getByPlaceholder('EXECUTE RUNBOOK PSPOUTAG').fill('EXECUTE RUNBOOK');
    await expect(article.getByRole('button', { name: 'Marquer execute' })).toBeDisabled();

    await article.getByPlaceholder('EXECUTE RUNBOOK PSPOUTAG').fill('EXECUTE RUNBOOK PSPOUTAG');
    await expect(article.getByRole('button', { name: 'Marquer execute' })).toBeEnabled();
    await article.getByRole('button', { name: 'Marquer execute' }).click();

    await expect(page.getByText('internal_admin.runbook.executed')).toBeVisible();
    expect(posts).toContainEqual(
      expect.objectContaining({
        confirmCode: 'EXECUTE RUNBOOK PSPOUTAG',
        hasIdempotencyKey: true,
        path: `/workspaces/${ids.workspaceId}/billing/admin/runbooks/psp-outage/execute`,
        secondApproverPrincipalId: ids.secondApproverId,
        secondApproverRole: 'platform_admin',
      }),
    );
  });

  test('Audit timeline renders evidence diff, hash status and target links', async ({ page }) => {
    await page.goto('/#audit');
    const section = page.locator('section#audit');

    await expect(section.getByText('internal_admin.workspace.suspend')).toBeVisible();
    await expect(section.getByText('linked')).toBeVisible();
    await expect(section.getByText('Hash: abcdef123456')).toBeVisible();
    await expect(section.getByText('Avant: active')).toBeVisible();
    await expect(section.getByText('Apres: suspended')).toBeVisible();

    await section.getByRole('button', { name: 'Open workspace' }).click();
    await expect(page).toHaveURL(/#workspace-detail/u);
  });

  test('Audit timeline exports an evidence package with operator context', async ({ page }) => {
    const exports: string[] = [];
    await page.route('**/admin/audit-evidence/export**', async (route) => {
      exports.push(route.request().headers()['x-nvbes-actor-principal-id'] ?? '');
      await route.fulfill({
        contentType: 'application/json',
        body: JSON.stringify({
          export_id: '99999999-9999-4999-8999-999999999999',
          generated_at: '2026-06-24T10:05:00Z',
          tenant_id: ids.tenantId,
          workspace_id: ids.workspaceId,
          event_count: 0,
          filters: { action: null, target_type: null, q: null, limit: 500 },
          hash_chain: {
            head_event_hash: null,
            tail_event_hash: null,
            anomaly_count: 0,
            linked_count: 0,
          },
          events: [],
        }),
      });
    });

    await page.goto('/#audit');
    await page.locator('section#audit').getByRole('button', { name: 'Export evidence' }).click();

    await expect.poll(() => exports).toEqual([ids.actorId]);
  });
});
