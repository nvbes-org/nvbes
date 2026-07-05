export * from './billing.client';
export * from './billing.provider';
export * from './billing.schemas';

import { BillingClient } from './billing.client';

export const billingClient = new BillingClient();

export function createBillingClient(
  options?: import('./billing.client').BillingClientOptions,
): BillingClient {
  return new BillingClient(options);
}
