import { describe, expect, it } from 'vite-plus/test';
import {
  billingProviderCodes,
  isBillingProviderCode,
  paymentMethodUpdateFlows,
} from './billing.provider';

describe('billing provider codes', () => {
  it('lists supported billing provider codes', () => {
    expect(billingProviderCodes).toEqual(['stripe', 'mollie', 'cb']);
  });

  it('narrows supported billing provider codes', () => {
    expect(isBillingProviderCode('stripe')).toBe(true);
    expect(isBillingProviderCode('mollie')).toBe(true);
    expect(isBillingProviderCode('cb')).toBe(true);
    expect(isBillingProviderCode('paypal')).toBe(false);
    expect(isBillingProviderCode('')).toBe(false);
  });

  it('lists supported payment method update flows', () => {
    expect(paymentMethodUpdateFlows).toEqual([
      'nvbes_provider_redirect',
      'provider_portal_unavailable',
    ]);
  });
});
