import type { Page } from '@playwright/test';

const TARGET_PRINCIPAL_ID = '00000000-0000-4000-8000-000000000002';

export async function installAccountConsentSwitchScenario(page: Page) {
  let releaseTargetConsent: (() => void) | undefined;
  const targetConsentRequested = new Promise<void>((resolve) => {
    releaseTargetConsent = resolve;
  });

  await page.route('**/auth/accounts**', async (route) => {
    await route.fulfill({
      json: { accounts: [account('1', 'Compte A'), account('2', 'Compte B')] },
    });
  });
  await page.route('**/auth/me**', async (route) => {
    await route.fulfill({
      json: {
        user: account('2', 'Compte B').user,
        current_tenant_id: null,
        current_organization_id: null,
        current_workspace_id: null,
        current_workspace_region: null,
      },
    });
  });
  await page.route('**/legal/consents**', async (route) => {
    await targetConsentRequested;
    await route.fulfill({
      json: {
        consents: [
          {
            id: '00000000-0000-4000-8000-000000000102',
            principal_id: TARGET_PRINCIPAL_ID,
            consent_type: 'analytics_product_analytics',
            document_version: 'cookie-notice-2026-07-20',
            granted_at: new Date().toISOString(),
            revoked_at: null,
          },
        ],
        next_cursor: null,
        has_more: false,
      },
    });
  });
  await page.route(/^https?:\/\/[^/]+\/switch-complete(?:\?.*)?$/u, async (route) => {
    await route.fulfill({
      contentType: 'text/html',
      body: '<!doctype html><html lang="fr"><title>Application cible</title></html>',
    });
  });

  return {
    releaseTargetConsent: () => releaseTargetConsent?.(),
  };
}

function account(authuser: string, displayName: string) {
  const suffix = authuser.padStart(12, '0');
  return {
    authuser,
    status: 'active',
    message: null,
    user: {
      id: authuser === '2' ? TARGET_PRINCIPAL_ID : '00000000-0000-4000-8000-000000000001',
      email: `account-${authuser}@example.test`,
      display_name: displayName,
      email_verified: true,
      mfa_enabled: false,
      created_at: '2026-08-02T12:00:00.000Z',
    },
    session: {
      id: `00000000-0000-4000-8000-${suffix}`,
      tenant_id: null,
      organization_id: null,
      workspace_id: null,
      workspace_region: null,
      created_at: '2026-08-02T12:00:00.000Z',
      last_seen_at: '2026-08-02T12:00:00.000Z',
      expires_at: '2026-08-03T12:00:00.000Z',
      revoked_at: null,
      ip: null,
      geo_country_code: null,
      user_agent: null,
      client: null,
      device_id: null,
      device_trust_level: null,
      device_trust_score: null,
      risk_score: null,
      risk_decision: null,
      risk_confirmed_at: null,
      current: authuser === '1',
    },
  };
}
