import { z } from 'zod';
import { BillingProviderCodeSchema, PaymentMethodUpdateFlowSchema } from './billing.provider';

const BillingRedirectSchema = z.object({
  url: z.string().url(),
});

export const BillingCheckoutRedirectSchema = BillingRedirectSchema.extend({
  provider: BillingProviderCodeSchema.optional(),
  session_id: z.string().optional(),
});

export const BillingPortalRedirectSchema = BillingRedirectSchema.extend({
  provider: BillingProviderCodeSchema.optional(),
});

export const BillingPortalCapabilitiesSchema = z.object({
  exposes_provider_secret_ids: z.boolean(),
  payment_method_update_flow: PaymentMethodUpdateFlowSchema,
  payment_method_changes_delegated_to_provider: z.boolean(),
  automatically_updates_payment_method_references: z.boolean(),
  shows_canonical_invoices: z.boolean(),
  shows_credits: z.boolean(),
});

export const BillingPortalInvoiceProviderSchema = z.object({
  provider: BillingProviderCodeSchema,
  status: z.string(),
  invoice_number: z.string().nullable().optional(),
  pdf_available: z.boolean(),
});

export const BillingPortalInvoiceSchema = z.object({
  invoice_id: z.string(),
  invoice_number: z.string().nullable().optional(),
  canonical_pdf_url: z.string(),
  status: z.string(),
  total_minor: z.number(),
  currency: z.string(),
  issued_at: z.string().nullable().optional(),
  due_at: z.string().nullable().optional(),
  paid_at: z.string().nullable().optional(),
  providers: z.array(BillingPortalInvoiceProviderSchema),
});

export const BillingPortalCreditSchema = z.object({
  amount_minor: z.number(),
  currency: z.string(),
});

export const BillingPortalPaymentMethodProviderSchema = z.object({
  provider: BillingProviderCodeSchema,
  status: z.string(),
  mandate_status: z.string(),
  reusable: z.boolean(),
});

export const BillingPortalPaymentMethodSchema = z.object({
  payment_method_id: z.string(),
  method_type: z.string(),
  display_label: z.string().nullable().optional(),
  brand: z.string().nullable().optional(),
  last4: z.string().nullable().optional(),
  exp_month: z.number().nullable().optional(),
  exp_year: z.number().nullable().optional(),
  funding: z.string().nullable().optional(),
  issuer_country: z.string().nullable().optional(),
  status: z.string(),
  is_primary: z.boolean(),
  providers: z.array(BillingPortalPaymentMethodProviderSchema),
});

export const BillingPortalSubscriptionProviderSchema = z.object({
  provider: BillingProviderCodeSchema,
  status: z.string(),
  primary: z.boolean(),
  fallback_eligible: z.boolean(),
  activated_at: z.string().nullable().optional(),
  deactivated_at: z.string().nullable().optional(),
});

export const BillingPortalViewSchema = z.object({
  plan_code: z.string(),
  provider: BillingProviderCodeSchema,
  payment_method_update_flow: PaymentMethodUpdateFlowSchema,
  payment_method_changes_delegated_to_provider: z.boolean(),
  automatically_updates_payment_method_references: z.boolean(),
  exposes_provider_secret_ids: z.boolean(),
  invoices: z.array(BillingPortalInvoiceSchema),
  credits: z.array(BillingPortalCreditSchema),
  payment_methods: z.array(BillingPortalPaymentMethodSchema),
  subscriptions: z.array(BillingPortalSubscriptionProviderSchema),
});

const BillingUsageLineSchema = z.object({
  included_quantity: z.number(),
  used_quantity: z.number(),
  billable_quantity: z.number(),
  unit: z.string(),
});

export const BillingUsageSchema = z.object({
  workspace_id: z.string(),
  period_start: z.string(),
  period_end: z.string(),
  storage: BillingUsageLineSchema,
  seats: BillingUsageLineSchema,
  bandwidth_out_bytes_month: z.number(),
});

export const ProductEntitlementsSchema = z.object({
  can_upload: z.boolean(),
  can_create_share_links: z.boolean(),
  included_storage_bytes: z.number(),
  included_users: z.number(),
  max_share_links: z.number(),
  max_share_link_ttl_days: z.number(),
  audit_level: z.string(),
  api_key_limit: z.number(),
  billing_locked: z.boolean(),
});

const PlanViewSchema = z.object({
  code: z.string(),
  included_storage_gb: z.number(),
  included_users: z.number(),
  retention_days: z.number(),
  max_share_links: z.number(),
  audit_level: z.string(),
  max_share_link_ttl_days: z.number(),
  monthly_price_cents: z.number(),
  currency: z.string(),
});

const SubscriptionViewSchema = z.object({
  status: z.string(),
  billing_provider: BillingProviderCodeSchema,
  current_period_start: z.string().nullable().optional(),
  current_period_end: z.string().nullable().optional(),
  trial_ends_at: z.string().nullable().optional(),
});

export const BillingOverviewSchema = z.object({
  workspace_id: z.string(),
  plan: PlanViewSchema,
  subscription: SubscriptionViewSchema,
  billing_account: z.object({
    billing_email: z.string().nullable().optional(),
    country: z.string().nullable().optional(),
    customer_type: z.string(),
    vat_number: z.string().nullable().optional(),
    tax_exempt_status: z.string().nullable().optional(),
  }),
  entitlements: ProductEntitlementsSchema,
  invoice_estimate: z.object({
    workspace_id: z.string(),
    billing_period_start: z.string(),
    billing_period_end: z.string(),
    base_amount_cents: z.number(),
    storage_overage_amount_cents: z.number(),
    seat_overage_amount_cents: z.number(),
    estimated_amount_cents: z.number(),
    currency: z.string(),
  }),
});

export type BillingCheckoutRedirect = z.infer<typeof BillingCheckoutRedirectSchema>;
export type BillingPortalRedirect = z.infer<typeof BillingPortalRedirectSchema>;
export type BillingPortalCapabilities = z.infer<typeof BillingPortalCapabilitiesSchema>;
export type BillingPortalInvoiceProvider = z.infer<typeof BillingPortalInvoiceProviderSchema>;
export type BillingPortalInvoice = z.infer<typeof BillingPortalInvoiceSchema>;
export type BillingPortalCredit = z.infer<typeof BillingPortalCreditSchema>;
export type BillingPortalPaymentMethodProvider = z.infer<
  typeof BillingPortalPaymentMethodProviderSchema
>;
export type BillingPortalPaymentMethod = z.infer<typeof BillingPortalPaymentMethodSchema>;
export type BillingPortalSubscriptionProvider = z.infer<
  typeof BillingPortalSubscriptionProviderSchema
>;
export type BillingPortalView = z.infer<typeof BillingPortalViewSchema>;
export type BillingUsageLine = z.infer<typeof BillingUsageLineSchema>;
export type BillingUsage = z.infer<typeof BillingUsageSchema>;
export type ProductEntitlements = z.infer<typeof ProductEntitlementsSchema>;
export type BillingOverview = z.infer<typeof BillingOverviewSchema>;

export const BillingPlanSchema = z.object({
  plan_code: z.string(),
  name: z.string(),
  stripe_price_id: z.string(),
  currency: z.string(),
  amount_cents: z.number(),
  billing_interval: z.string(),
});
export type BillingPlan = z.infer<typeof BillingPlanSchema>;

export const BillingOverviewV1Schema = z.object({
  account_id: z.string(),
  customer_id: z.string().nullable().optional(),
  plan_code: z.string(),
  status: z.string(),
  current_period_end: z.string().nullable().optional(),
  cancel_at_period_end: z.boolean(),
});
export type BillingOverviewV1 = z.infer<typeof BillingOverviewV1Schema>;

export const BillingOperatorOverviewSchema = z.object({
  active_subscriptions: z.number(),
  open_checkouts: z.number(),
  pending_reconciliations: z.number(),
  pending_outbox: z.number(),
});
export type BillingOperatorOverview = z.infer<typeof BillingOperatorOverviewSchema>;

export const BillingReconciliationItemSchema = z.object({
  id: z.string(),
  source_event_id: z.string().nullable().optional(),
  account_id: z.string().nullable().optional(),
  reason: z.string(),
  details: z.record(z.string(), z.unknown()),
  status: z.string(),
  resolved_by: z.string().nullable().optional(),
  resolved_at: z.string().nullable().optional(),
  resolution_notes: z.string().nullable().optional(),
  created_at: z.string(),
});
export type BillingReconciliationItem = z.infer<typeof BillingReconciliationItemSchema>;
