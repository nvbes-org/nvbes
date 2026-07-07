import { z } from 'zod';
import { identityHttpClient } from './identity.http';

const NullableStringSchema = z.string().nullable();

const AccountBillingEntitlementsSchema = z.object({
  included_storage_gb: z.number(),
  included_users: z.number(),
  retention_days: z.number(),
  max_share_links: z.number(),
  audit_level: z.string(),
});

export const AccountBillingOverviewSchema = z.object({
  workspace_id: z.string(),
  plan_code: z.string(),
  subscription_status: z.string(),
  billing_provider: z.string(),
  current_period_start: NullableStringSchema.optional(),
  current_period_end: NullableStringSchema.optional(),
  entitlements: AccountBillingEntitlementsSchema,
});

const AccountBillingProviderReferenceSchema = z.object({
  provider: z.string(),
  status: z.string(),
  primary: z.boolean(),
  fallback_eligible: z.boolean(),
});

const AccountBillingInvoiceSchema = z.object({
  invoice_id: z.string(),
  invoice_number: NullableStringSchema.optional(),
  status: z.string(),
  total_minor: z.number(),
  currency: z.string(),
  issued_at: NullableStringSchema.optional(),
  providers: z.array(AccountBillingProviderReferenceSchema),
});

const AccountBillingPaymentMethodSchema = z.object({
  payment_method_id: z.string(),
  brand: NullableStringSchema.optional(),
  last4: NullableStringSchema.optional(),
  exp_month: z.number().nullable().optional(),
  exp_year: z.number().nullable().optional(),
  providers: z.array(AccountBillingProviderReferenceSchema),
});

const AccountBillingSubscriptionSchema = z.object({
  subscription_id: z.string(),
  plan_code: NullableStringSchema.optional(),
  status: z.string(),
  providers: z.array(AccountBillingProviderReferenceSchema),
});

export const AccountBillingPortalViewSchema = z.object({
  workspace_id: z.string(),
  provider: z.string(),
  invoices: z.array(AccountBillingInvoiceSchema),
  payment_methods: z.array(AccountBillingPaymentMethodSchema),
  subscriptions: z.array(AccountBillingSubscriptionSchema),
});

export const AccountBillingSessionSchema = z.object({
  url: z.string().url(),
  provider: z.string(),
  expires_at: NullableStringSchema.optional(),
});

export type AccountBillingOverview = z.infer<typeof AccountBillingOverviewSchema>;
export type AccountBillingPortalView = z.infer<typeof AccountBillingPortalViewSchema>;
export type AccountBillingSession = z.infer<typeof AccountBillingSessionSchema>;

export const accountBillingClient = {
  getOverview(workspaceId: string, options?: { signal?: AbortSignal }) {
    return identityHttpClient.get(
      `/account/billing/workspaces/${encodeURIComponent(workspaceId)}/overview`,
      AccountBillingOverviewSchema,
      options,
    );
  },
  getPortalView(workspaceId: string, options?: { signal?: AbortSignal }) {
    return identityHttpClient.get(
      `/account/billing/workspaces/${encodeURIComponent(workspaceId)}/portal`,
      AccountBillingPortalViewSchema,
      options,
    );
  },
  createCheckoutSession(workspaceId: string, planCode: string): Promise<AccountBillingSession> {
    return identityHttpClient.post(
      `/account/billing/workspaces/${encodeURIComponent(workspaceId)}/checkout`,
      AccountBillingSessionSchema,
      { plan_code: planCode },
    );
  },
  createPortalSession(workspaceId: string): Promise<AccountBillingSession> {
    return identityHttpClient.post(
      `/account/billing/workspaces/${encodeURIComponent(workspaceId)}/portal`,
      AccountBillingSessionSchema,
      {},
    );
  },
};
