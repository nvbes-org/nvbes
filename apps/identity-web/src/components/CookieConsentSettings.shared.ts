import type { CookieConsentState } from '../tracking-consent';

export const essentialVendors = ['⚡ nvbes Identity', '💳 Stripe (Paiements et Fraude)'];

export const posthogPurposes: Array<{
  key: keyof CookieConsentState['posthog'];
  label: string;
  description: string;
}> = [
  {
    key: 'productAnalytics',
    label: 'Analytics produit',
    description: 'Funnel produit pseudonymisé, sans emails, noms ou fichiers.',
  },
  {
    key: 'featureFlags',
    label: 'Feature flags',
    description: 'Expériences non critiques, jamais auth, sécurité ou billing.',
  },
  {
    key: 'autocaptureHeatmaps',
    label: 'Heatmaps & autocapture',
    description: 'Interactions masquées, désactivées sur les routes sensibles.',
  },
  {
    key: 'sessionReplay',
    label: 'Session replay',
    description: 'Replay masqué, bloqué sur auth, MFA, privacy, billing et fichiers.',
  },
  {
    key: 'surveysFeedback',
    label: 'Surveys & feedback',
    description: 'Feedback post-auth, jamais sur les pages sensibles.',
  },
  {
    key: 'errorTracking',
    label: 'PostHog error tracking',
    description: 'Capture navigateur scrubbed en parallèle de Sentry.',
  },
];

export function getConsentVendorCategory(
  vendor: keyof CookieConsentState['vendors'],
): keyof CookieConsentState['categories'] | undefined {
  if (vendor === 'sentry') {
    return 'performance';
  }

  if (vendor === 'posthog') {
    return 'analytics';
  }

  return undefined;
}
