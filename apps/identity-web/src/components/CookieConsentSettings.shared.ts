export const essentialVendors = ['nvbes Identity', 'nvbes Billing', 'Cloudflare (WAF/CDN)'];

export const analyticsVendors = [
  {
    key: 'posthog',
    label: 'PostHog',
    description: "Mesure d'audience produit, heatmaps, replay masqué et feature flags.",
  },
] as const;

export const performanceVendors = [
  {
    key: 'sentry',
    label: 'Sentry',
    description: "Suivi d'erreurs applicatives côté navigateur.",
  },
  {
    key: 'grafana',
    label: 'Grafana Labs',
    description: 'Observabilité technique, métriques et diagnostics de stabilité.',
  },
] as const;

export type ConsentVendorKey =
  | (typeof analyticsVendors)[number]['key']
  | (typeof performanceVendors)[number]['key'];

export const vendorCopy: Record<
  ConsentVendorKey,
  {
    label: string;
    description: string;
  }
> = {
  posthog: analyticsVendors[0],
  sentry: performanceVendors[0],
  grafana: performanceVendors[1],
};

export function getConsentVendorCategory(vendor: string): 'analytics' | 'performance' | undefined {
  if (vendor === 'sentry' || vendor === 'grafana') {
    return 'performance';
  }

  if (vendor === 'posthog') {
    return 'analytics';
  }

  return undefined;
}
