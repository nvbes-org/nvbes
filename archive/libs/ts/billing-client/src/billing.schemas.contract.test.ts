import { describe, expect, it } from 'vite-plus/test';
import {
  BillingOverviewSchema,
  BillingPortalCapabilitiesSchema,
  BillingPortalCreditSchema,
  BillingPortalInvoiceSchema,
  BillingPortalRedirectSchema,
  BillingUsageSchema,
  ProductEntitlementsSchema,
} from './billing.schemas';

function expectExactParse(schema: { parse(value: unknown): unknown }, value: unknown) {
  expect(schema.parse(value)).toEqual(value);
}

describe('billing schema contracts', () => {
  it('preserves the complete provider redirect contract', () => {
    expectExactParse(BillingPortalRedirectSchema, {
      provider: 'stripe',
      url: 'https://billing.test/portal',
    });
  });

  it('preserves every portal capability', () => {
    expectExactParse(BillingPortalCapabilitiesSchema, {
      automatically_updates_payment_method_references: true,
      exposes_provider_secret_ids: false,
      payment_method_changes_delegated_to_provider: true,
      payment_method_update_flow: 'nvbes_provider_redirect',
      shows_canonical_invoices: true,
      shows_credits: true,
    });
  });

  it('preserves canonical invoice and provider facts', () => {
    expectExactParse(BillingPortalInvoiceSchema, {
      canonical_pdf_url: '/billing/invoices/invoice-1.pdf',
      currency: 'eur',
      due_at: '2026-09-20T00:00:00Z',
      invoice_id: 'invoice-1',
      invoice_number: '2026-001',
      issued_at: '2026-09-10T00:00:00Z',
      paid_at: null,
      providers: [
        {
          invoice_number: 'STRIPE-001',
          pdf_available: true,
          provider: 'stripe',
          status: 'open',
        },
      ],
      status: 'open',
      total_minor: 1200,
    });
  });

  it('preserves a credit amount and currency', () => {
    expectExactParse(BillingPortalCreditSchema, { amount_minor: 300, currency: 'eur' });
  });

  it('preserves usage totals and both usage lines', () => {
    expectExactParse(BillingUsageSchema, {
      bandwidth_out_bytes_month: 1024,
      period_end: '2026-10-01T00:00:00Z',
      period_start: '2026-09-01T00:00:00Z',
      seats: {
        billable_quantity: 1,
        included_quantity: 2,
        unit: 'seat',
        used_quantity: 3,
      },
      storage: {
        billable_quantity: 0,
        included_quantity: 10,
        unit: 'gb',
        used_quantity: 4,
      },
      workspace_id: 'workspace-1',
    });
  });

  it('preserves every entitlement used by product authorization', () => {
    expectExactParse(ProductEntitlementsSchema, {
      api_key_limit: 2,
      audit_level: 'standard',
      billing_locked: false,
      can_create_share_links: true,
      can_upload: true,
      included_storage_bytes: 10_000,
      included_users: 3,
      max_share_links: 20,
      max_share_link_ttl_days: 30,
    });
  });

  it('preserves the complete legacy overview contract', () => {
    expectExactParse(BillingOverviewSchema, {
      billing_account: {
        billing_email: 'billing@example.test',
        country: 'FR',
        customer_type: 'individual',
        tax_exempt_status: null,
        vat_number: null,
      },
      entitlements: {
        api_key_limit: 2,
        audit_level: 'standard',
        billing_locked: false,
        can_create_share_links: true,
        can_upload: true,
        included_storage_bytes: 10_000,
        included_users: 3,
        max_share_links: 20,
        max_share_link_ttl_days: 30,
      },
      invoice_estimate: {
        base_amount_cents: 1000,
        billing_period_end: '2026-10-01T00:00:00Z',
        billing_period_start: '2026-09-01T00:00:00Z',
        currency: 'eur',
        estimated_amount_cents: 1200,
        seat_overage_amount_cents: 200,
        storage_overage_amount_cents: 0,
        workspace_id: 'workspace-1',
      },
      plan: {
        audit_level: 'standard',
        code: 'team',
        currency: 'eur',
        included_storage_gb: 10,
        included_users: 3,
        max_share_link_ttl_days: 30,
        max_share_links: 20,
        monthly_price_cents: 1000,
        retention_days: 30,
      },
      subscription: {
        billing_provider: 'stripe',
        current_period_end: '2026-10-01T00:00:00Z',
        current_period_start: '2026-09-01T00:00:00Z',
        status: 'active',
        trial_ends_at: null,
      },
      workspace_id: 'workspace-1',
    });
  });
});
