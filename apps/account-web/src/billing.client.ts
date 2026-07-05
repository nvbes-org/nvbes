import { createBillingClient } from '@nvbes/billing-client';

export function resolveBillingApiBaseUrl(): string {
  const configuredBaseUrl = import.meta.env.VITE_BILLING_SERVICE_BASE_URL?.trim();
  if (configuredBaseUrl) return configuredBaseUrl;
  if (import.meta.env.DEV) return 'http://localhost:4020';
  throw new Error('VITE_BILLING_SERVICE_BASE_URL is required outside local development.');
}

export const billingServiceBaseUrl = resolveBillingApiBaseUrl();

export const billingClient = createBillingClient({
  baseUrl: billingServiceBaseUrl,
});
