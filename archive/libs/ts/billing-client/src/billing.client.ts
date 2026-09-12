import { createHttpClient, type HttpClient } from '@nvbes/http-client';
import {
  BillingCheckoutRedirectSchema,
  BillingOverviewSchema,
  BillingPortalCapabilitiesSchema,
  BillingPortalRedirectSchema,
  BillingPortalInvoiceSchema,
  BillingPortalPaymentMethodSchema,
  BillingPortalSubscriptionProviderSchema,
  BillingPortalViewSchema,
  BillingUsageSchema,
  ProductEntitlementsSchema,
  BillingPlanSchema,
  BillingOverviewV1Schema,
  BillingOperatorOverviewSchema,
  BillingReconciliationItemSchema,
  type BillingCheckoutRedirect,
  type BillingOverview,
  type BillingPortalCapabilities,
  type BillingPortalInvoice,
  type BillingPortalPaymentMethod,
  type BillingPortalRedirect,
  type BillingPortalSubscriptionProvider,
  type BillingPortalView,
  type BillingUsage,
  type ProductEntitlements,
  type BillingPlan,
  type BillingOverviewV1,
  type BillingOperatorOverview,
  type BillingReconciliationItem,
} from './billing.schemas';

export type RequestOptions = { signal?: AbortSignal };

export type BillingClientOptions = {
  baseUrl?: string;
  http?: HttpClient;
};

export class BillingClient {
  private readonly http: HttpClient;

  constructor(options: BillingClientOptions = {}) {
    this.http =
      options.http ??
      createHttpClient({
        baseUrl: options.baseUrl,
        credentials: 'include',
      });
  }

  createCheckout(workspaceId: string, planCode: string): Promise<string> {
    return this.createCheckoutSession(workspaceId, planCode).then((response) => response.url);
  }

  createCheckoutSession(workspaceId: string, planCode: string): Promise<BillingCheckoutRedirect> {
    return this.http.post(
      `/workspaces/${workspaceId}/billing/checkout`,
      BillingCheckoutRedirectSchema,
      {
        plan_code: planCode,
      },
    );
  }

  createPortal(workspaceId: string): Promise<string> {
    return this.createPortalSession(workspaceId).then((response) => response.url);
  }

  createPortalSession(workspaceId: string): Promise<BillingPortalRedirect> {
    return this.http.post(
      `/workspaces/${workspaceId}/billing/portal`,
      BillingPortalRedirectSchema,
      {},
    );
  }

  getPortalCapabilities(options?: RequestOptions): Promise<BillingPortalCapabilities> {
    return this.http.get('/billing/portal/capabilities', BillingPortalCapabilitiesSchema, options);
  }

  getPortalView(workspaceId: string, options?: RequestOptions): Promise<BillingPortalView> {
    return this.http.get(
      `/workspaces/${workspaceId}/billing/portal/view`,
      BillingPortalViewSchema,
      options,
    );
  }

  getOverview(workspaceId: string, options?: RequestOptions): Promise<BillingOverview> {
    return this.http.get(
      `/workspaces/${workspaceId}/billing/overview`,
      BillingOverviewSchema,
      options,
    );
  }

  getUsage(workspaceId: string, options?: RequestOptions): Promise<BillingUsage> {
    return this.http.get(`/workspaces/${workspaceId}/billing/usage`, BillingUsageSchema, options);
  }

  getEntitlements(workspaceId: string, options?: RequestOptions): Promise<ProductEntitlements> {
    return this.http.get(
      `/workspaces/${workspaceId}/billing/entitlements`,
      ProductEntitlementsSchema,
      options,
    );
  }

  getInvoices(workspaceId: string, options?: RequestOptions): Promise<BillingPortalInvoice[]> {
    return this.http.get(
      `/workspaces/${workspaceId}/billing/invoices`,
      BillingPortalInvoiceSchema.array(),
      options,
    );
  }

  getCards(workspaceId: string, options?: RequestOptions): Promise<BillingPortalPaymentMethod[]> {
    return this.http.get(
      `/workspaces/${workspaceId}/billing/cards`,
      BillingPortalPaymentMethodSchema.array(),
      options,
    );
  }

  getSubscriptions(
    workspaceId: string,
    options?: RequestOptions,
  ): Promise<BillingPortalSubscriptionProvider[]> {
    return this.http.get(
      `/workspaces/${workspaceId}/billing/subscriptions`,
      BillingPortalSubscriptionProviderSchema.array(),
      options,
    );
  }

  getPlans(options?: RequestOptions): Promise<BillingPlan[]> {
    return this.http.get('/billing/plans', BillingPlanSchema.array(), options);
  }

  getOverviewV1(workspaceId: string, options?: RequestOptions): Promise<BillingOverviewV1> {
    return this.http.get(
      `/workspaces/${workspaceId}/billing/overview`,
      BillingOverviewV1Schema,
      options,
    );
  }

  getOperatorOverview(options?: RequestOptions): Promise<BillingOperatorOverview> {
    return this.http.get('/operator/billing/overview', BillingOperatorOverviewSchema, options);
  }

  listReconciliations(options?: RequestOptions): Promise<BillingReconciliationItem[]> {
    return this.http.get(
      '/operator/billing/reconciliations',
      BillingReconciliationItemSchema.array(),
      options,
    );
  }

  resolveReconciliation(
    id: string,
    notes: string,
    options?: RequestOptions,
  ): Promise<BillingReconciliationItem> {
    return this.http.post(
      `/operator/billing/reconciliations/${id}/resolve`,
      BillingReconciliationItemSchema,
      { resolution_notes: notes },
      options,
    );
  }
}
