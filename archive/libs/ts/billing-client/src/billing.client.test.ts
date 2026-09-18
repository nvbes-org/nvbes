import { describe, expect, it } from 'vite-plus/test';
import { BillingClient } from './billing.client';
import { billingClient, createBillingClient } from './index';
import {
  BillingCheckoutRedirectSchema,
  BillingOperatorOverviewSchema,
  BillingOverviewV1Schema,
  BillingPlanSchema,
  BillingReconciliationItemSchema,
} from './billing.schemas';

describe('BillingClient and V1 Schemas', () => {
  it('instantiates the BillingClient singleton and factory', () => {
    expect(billingClient).toBeInstanceOf(BillingClient);
    expect(createBillingClient()).toBeInstanceOf(BillingClient);
  });

  it('validates checkout redirect with session_id', () => {
    const valid = {
      url: 'https://checkout.stripe.com/c/pay/cs_test_123',
      session_id: 'cs_test_123',
    };
    const parsed = BillingCheckoutRedirectSchema.parse(valid);
    expect(parsed.url).toBe(valid.url);
    expect(parsed.session_id).toBe('cs_test_123');
  });

  it('validates V1 billing plan schema', () => {
    const plan = {
      plan_code: 'standard_monthly',
      name: 'Standard',
      stripe_price_id: 'price_test_standard',
      currency: 'eur',
      amount_cents: 1000,
      billing_interval: 'month',
    };
    const parsed = BillingPlanSchema.parse(plan);
    expect(parsed.plan_code).toBe('standard_monthly');
    expect(parsed.amount_cents).toBe(1000);
  });

  it('validates V1 workspace billing overview schema', () => {
    const overview = {
      account_id: '00000000-0000-0000-0000-000000000001',
      customer_id: 'cus_test_abc123',
      plan_code: 'standard_monthly',
      status: 'active',
      current_period_end: '2026-10-01T00:00:00Z',
      cancel_at_period_end: false,
    };
    const parsed = BillingOverviewV1Schema.parse(overview);
    expect(parsed.account_id).toBe(overview.account_id);
    expect(parsed.status).toBe('active');
  });

  it('validates operator overview and reconciliation item schemas', () => {
    const operatorOverview = {
      active_subscriptions: 10,
      open_checkouts: 2,
      pending_reconciliations: 1,
      pending_outbox: 0,
    };
    const parsedOverview = BillingOperatorOverviewSchema.parse(operatorOverview);
    expect(parsedOverview.active_subscriptions).toBe(10);

    const item = {
      id: '00000000-0000-0000-0000-000000000002',
      source_event_id: 'evt_test_123',
      account_id: '00000000-0000-0000-0000-000000000001',
      reason: 'anomaly',
      details: { foo: 'bar' },
      status: 'pending',
      resolved_by: null,
      resolved_at: null,
      resolution_notes: null,
      created_at: '2026-09-01T12:00:00Z',
    };
    const parsedItem = BillingReconciliationItemSchema.parse(item);
    expect(parsedItem.id).toBe(item.id);
    expect(parsedItem.status).toBe('pending');
  });
});
