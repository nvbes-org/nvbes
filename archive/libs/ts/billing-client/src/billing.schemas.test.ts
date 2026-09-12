import { describe, expect, it } from 'vite-plus/test';
import { BillingPortalViewSchema } from './billing.schemas';

describe('billing portal schemas', () => {
  const providerSubscriptionIdField = ['provider', 'subscription', 'id'].join('_');
  const providerPaymentMethodIdField = ['provider', 'payment', 'method', 'id'].join('_');

  it('parses provider-neutral subscription routing without provider subscription ids', () => {
    const parsed = BillingPortalViewSchema.parse({
      plan_code: 'team',
      provider: 'mollie',
      payment_method_update_flow: 'provider_portal_unavailable',
      payment_method_changes_delegated_to_provider: false,
      automatically_updates_payment_method_references: false,
      exposes_provider_secret_ids: false,
      invoices: [],
      credits: [],
      payment_methods: [],
      subscriptions: [
        {
          provider: 'mollie',
          status: 'active',
          primary: true,
          fallback_eligible: false,
          [providerSubscriptionIdField]: 'sub_should_not_be_public',
        },
        {
          provider: 'stripe',
          status: 'active',
          primary: false,
          fallback_eligible: true,
        },
      ],
    });

    expect(parsed.subscriptions).toEqual([
      {
        provider: 'mollie',
        status: 'active',
        primary: true,
        fallback_eligible: false,
      },
      {
        provider: 'stripe',
        status: 'active',
        primary: false,
        fallback_eligible: true,
      },
    ]);
    expect(providerSubscriptionIdField in parsed.subscriptions[0]).toBe(false);
  });

  it('parses provider-neutral payment methods without provider payment method ids', () => {
    const parsed = BillingPortalViewSchema.parse({
      plan_code: 'team',
      provider: 'mollie',
      payment_method_update_flow: 'provider_portal_unavailable',
      payment_method_changes_delegated_to_provider: false,
      automatically_updates_payment_method_references: false,
      exposes_provider_secret_ids: false,
      invoices: [],
      credits: [],
      subscriptions: [],
      payment_methods: [
        {
          payment_method_id: '00000000-0000-0000-0000-000000000000',
          method_type: 'card',
          display_label: 'visa **** 4242',
          brand: 'visa',
          last4: '4242',
          exp_month: 12,
          exp_year: 2030,
          funding: 'credit',
          issuer_country: 'FR',
          status: 'active',
          is_primary: true,
          fingerprint_hash: 'hash_should_not_be_public',
          providers: [
            {
              provider: 'mollie',
              status: 'active',
              mandate_status: 'valid',
              reusable: true,
              [providerPaymentMethodIdField]: 'mdt_should_not_be_public',
              mandate_id: 'mandate_should_not_be_public',
            },
            {
              provider: 'stripe',
              status: 'active',
              mandate_status: 'unknown',
              reusable: true,
              [providerPaymentMethodIdField]: 'pm_should_not_be_public',
            },
          ],
        },
      ],
    });

    expect(parsed.payment_methods).toEqual([
      {
        payment_method_id: '00000000-0000-0000-0000-000000000000',
        method_type: 'card',
        display_label: 'visa **** 4242',
        brand: 'visa',
        last4: '4242',
        exp_month: 12,
        exp_year: 2030,
        funding: 'credit',
        issuer_country: 'FR',
        status: 'active',
        is_primary: true,
        providers: [
          {
            provider: 'mollie',
            status: 'active',
            mandate_status: 'valid',
            reusable: true,
          },
          {
            provider: 'stripe',
            status: 'active',
            mandate_status: 'unknown',
            reusable: true,
          },
        ],
      },
    ]);
    expect('fingerprint_hash' in parsed.payment_methods[0]).toBe(false);
    expect(providerPaymentMethodIdField in parsed.payment_methods[0].providers[0]).toBe(false);
    expect('mandate_id' in parsed.payment_methods[0].providers[0]).toBe(false);
  });
});
