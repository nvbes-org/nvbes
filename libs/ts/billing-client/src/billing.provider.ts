import { z } from 'zod';

export const billingProviderCodes = ['stripe', 'mollie', 'cb'] as const;

export const BillingProviderCodeSchema = z.enum(billingProviderCodes);

export type BillingProviderCode = z.infer<typeof BillingProviderCodeSchema>;

export const paymentMethodUpdateFlows = [
  'nvbes_provider_redirect',
  'provider_portal_unavailable',
] as const;

export const PaymentMethodUpdateFlowSchema = z.enum(paymentMethodUpdateFlows);

export type PaymentMethodUpdateFlow = z.infer<typeof PaymentMethodUpdateFlowSchema>;

export function isBillingProviderCode(value: string): value is BillingProviderCode {
  return billingProviderCodes.includes(value as BillingProviderCode);
}
