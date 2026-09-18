import { describe, expect, it, vi } from 'vite-plus/test';
import { BillingClient, type BillingClientOptions, type RequestOptions } from './billing.client';
import {
  BillingCheckoutRedirectSchema,
  BillingOperatorOverviewSchema,
  BillingOverviewSchema,
  BillingOverviewV1Schema,
  BillingPlanSchema,
  BillingPortalCapabilitiesSchema,
  BillingPortalInvoiceSchema,
  BillingPortalPaymentMethodSchema,
  BillingPortalRedirectSchema,
  BillingPortalSubscriptionProviderSchema,
  BillingPortalViewSchema,
  BillingReconciliationItemSchema,
  BillingUsageSchema,
  ProductEntitlementsSchema,
} from './billing.schemas';

function createHttpDouble() {
  const get = vi.fn().mockResolvedValue({});
  const post = vi.fn().mockResolvedValue({});
  const http = { get, post } as unknown as NonNullable<BillingClientOptions['http']>;
  return { get, http, post };
}

describe('BillingClient transport contract', () => {
  it('uses the configured origin with authenticated browser credentials', async () => {
    const fetchImpl = vi.fn().mockResolvedValue(
      new Response(JSON.stringify([]), {
        headers: { 'content-type': 'application/json' },
        status: 200,
      }),
    );
    vi.stubGlobal('fetch', fetchImpl);

    const client = new BillingClient({ baseUrl: 'https://billing.test' });
    await client.getPlans();

    expect(fetchImpl).toHaveBeenCalledOnce();
    expect(fetchImpl.mock.calls[0][0]).toBe('https://billing.test/billing/plans');
    expect(fetchImpl.mock.calls[0][1]).toMatchObject({ credentials: 'include', method: 'GET' });
    vi.unstubAllGlobals();
  });

  it('maps checkout and portal commands to their HTTP contracts', async () => {
    const { http, post } = createHttpDouble();
    const client = new BillingClient({ http });
    const signal = AbortSignal.abort();

    post
      .mockResolvedValueOnce({ url: 'https://billing.test/checkout', session_id: 'session-1' })
      .mockResolvedValueOnce({ url: 'https://billing.test/checkout-session' })
      .mockResolvedValueOnce({ url: 'https://billing.test/portal' })
      .mockResolvedValueOnce({ url: 'https://billing.test/portal-session' })
      .mockResolvedValueOnce({ id: 'reconciliation-1' });

    await expect(client.createCheckout('workspace-1', 'team')).resolves.toBe(
      'https://billing.test/checkout',
    );
    await client.createCheckoutSession('workspace-2', 'solo');
    await expect(client.createPortal('workspace-3')).resolves.toBe('https://billing.test/portal');
    await client.createPortalSession('workspace-4');
    await client.resolveReconciliation('reconciliation-1', 'resolved manually', { signal });

    expect(post.mock.calls).toEqual([
      [
        '/workspaces/workspace-1/billing/checkout',
        BillingCheckoutRedirectSchema,
        { plan_code: 'team' },
      ],
      [
        '/workspaces/workspace-2/billing/checkout',
        BillingCheckoutRedirectSchema,
        { plan_code: 'solo' },
      ],
      ['/workspaces/workspace-3/billing/portal', BillingPortalRedirectSchema, {}],
      ['/workspaces/workspace-4/billing/portal', BillingPortalRedirectSchema, {}],
      [
        '/operator/billing/reconciliations/reconciliation-1/resolve',
        BillingReconciliationItemSchema,
        { resolution_notes: 'resolved manually' },
        { signal },
      ],
    ]);
  });

  it('maps read operations to their paths, schemas and cancellation options', async () => {
    const { get, http } = createHttpDouble();
    const client = new BillingClient({ http });
    const options: RequestOptions = { signal: AbortSignal.abort() };

    await client.getPortalCapabilities(options);
    await client.getPortalView('workspace-1', options);
    await client.getOverview('workspace-1', options);
    await client.getUsage('workspace-1', options);
    await client.getEntitlements('workspace-1', options);
    await client.getInvoices('workspace-1', options);
    await client.getCards('workspace-1', options);
    await client.getSubscriptions('workspace-1', options);
    await client.getPlans(options);
    await client.getOverviewV1('workspace-1', options);
    await client.getOperatorOverview(options);
    await client.listReconciliations(options);

    const expected = [
      ['/billing/portal/capabilities', BillingPortalCapabilitiesSchema],
      ['/workspaces/workspace-1/billing/portal/view', BillingPortalViewSchema],
      ['/workspaces/workspace-1/billing/overview', BillingOverviewSchema],
      ['/workspaces/workspace-1/billing/usage', BillingUsageSchema],
      ['/workspaces/workspace-1/billing/entitlements', ProductEntitlementsSchema],
      ['/workspaces/workspace-1/billing/invoices', BillingPortalInvoiceSchema],
      ['/workspaces/workspace-1/billing/cards', BillingPortalPaymentMethodSchema],
      ['/workspaces/workspace-1/billing/subscriptions', BillingPortalSubscriptionProviderSchema],
      ['/billing/plans', BillingPlanSchema],
      ['/workspaces/workspace-1/billing/overview', BillingOverviewV1Schema],
      ['/operator/billing/overview', BillingOperatorOverviewSchema],
      ['/operator/billing/reconciliations', BillingReconciliationItemSchema],
    ] as const;

    expect(get).toHaveBeenCalledTimes(expected.length);
    for (const [index, [path, schema]] of expected.entries()) {
      const [actualPath, actualSchema, actualOptions] = get.mock.calls[index];
      expect(actualPath).toBe(path);
      expect(actualOptions).toBe(options);
      if (
        path.endsWith('/invoices') ||
        path.endsWith('/cards') ||
        path.endsWith('/subscriptions')
      ) {
        expect(actualSchema.safeParse([]).success).toBe(true);
      } else if (path === '/billing/plans' || path.endsWith('/reconciliations')) {
        expect(actualSchema.safeParse([]).success).toBe(true);
      } else {
        expect(actualSchema).toBe(schema);
      }
    }
  });
});
