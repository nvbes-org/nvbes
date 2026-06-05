import { useState } from 'react';
import {
  DEFAULT_CONSENT,
  getTrackingConsent,
  setTrackingConsent,
  type CookieConsentState,
} from '../tracking-consent';

type PostHogPurpose = keyof CookieConsentState['posthog'];

const ANALYTICS_POSTHOG_PURPOSES: PostHogPurpose[] = [
  'productAnalytics',
  'autocaptureHeatmaps',
  'sessionReplay',
  'surveysFeedback',
  'featureFlags',
];

function hasAnyPostHogPurpose(posthog: CookieConsentState['posthog']): boolean {
  return Object.values(posthog).some((value) => value);
}

function deriveConsentState(consent: CookieConsentState): CookieConsentState {
  return {
    categories: {
      essentials: true,
      analytics: ANALYTICS_POSTHOG_PURPOSES.some((purpose) => consent.posthog[purpose]),
      performance: consent.vendors.sentry || consent.posthog.errorTracking,
    },
    vendors: {
      stripe: true,
      identity: true,
      posthog: hasAnyPostHogPurpose(consent.posthog),
      sentry: consent.vendors.sentry,
    },
    posthog: { ...consent.posthog },
  };
}

function ConsentToggle({
  checked,
  onChange,
  label,
  description,
  large = false,
}: {
  checked: boolean;
  onChange: () => void;
  label: string;
  description: string;
  large?: boolean;
}) {
  const sizeClass = large ? 'w-9 h-5 after:h-4 after:w-4' : 'w-8 h-4 after:h-3 after:w-3';

  return (
    <div className="flex items-center justify-between gap-3">
      <div>
        <p className="text-xs font-medium text-foreground">{label}</p>
        <p className="text-[11px] text-muted-foreground">{description}</p>
      </div>
      <label className="relative inline-flex shrink-0 items-center cursor-pointer">
        <input type="checkbox" checked={checked} onChange={onChange} className="sr-only peer" />
        <div
          className={`${sizeClass} bg-muted peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-background after:border-border after:border after:rounded-full after:transition-all peer-checked:bg-primary`}
        />
      </label>
    </div>
  );
}

export function CookieConsentSettings() {
  const [cookieConsent, setCookieConsent] = useState<CookieConsentState>(
    () => getTrackingConsent() || DEFAULT_CONSENT,
  );

  const persistConsent = (next: CookieConsentState, source: string) => {
    const derived = deriveConsentState(next);
    setCookieConsent(derived);
    setTrackingConsent(derived, source);
  };

  const toggleCategory = (category: keyof CookieConsentState['categories']) => {
    if (category === 'essentials') return;

    const nextVal = !cookieConsent.categories[category];
    const nextVendors = { ...cookieConsent.vendors };
    const nextPosthog = { ...cookieConsent.posthog };

    if (category === 'analytics') {
      for (const purpose of ANALYTICS_POSTHOG_PURPOSES) {
        nextPosthog[purpose] = nextVal;
      }
    } else {
      nextVendors.sentry = nextVal;
      nextPosthog.errorTracking = nextVal;
    }

    persistConsent(
      {
        categories: { ...cookieConsent.categories, [category]: nextVal },
        vendors: nextVendors,
        posthog: nextPosthog,
      },
      `identity-web:account-privacy:${category}`,
    );
  };

  const toggleVendor = (vendor: keyof CookieConsentState['vendors']) => {
    if (vendor === 'stripe' || vendor === 'identity') return;

    const nextVal = !cookieConsent.vendors[vendor];
    const nextVendors = { ...cookieConsent.vendors, [vendor]: nextVal };
    const nextPosthog = { ...cookieConsent.posthog };

    if (vendor === 'posthog') {
      for (const purpose of Object.keys(nextPosthog) as PostHogPurpose[]) {
        nextPosthog[purpose] = nextVal;
      }
    } else {
      nextPosthog.errorTracking = nextPosthog.errorTracking && nextVal;
    }

    persistConsent(
      {
        categories: { ...cookieConsent.categories },
        vendors: nextVendors,
        posthog: nextPosthog,
      },
      `identity-web:account-privacy:${vendor}`,
    );
  };

  const togglePostHogPurpose = (purpose: PostHogPurpose) => {
    persistConsent(
      {
        categories: { ...cookieConsent.categories },
        vendors: { ...cookieConsent.vendors },
        posthog: {
          ...cookieConsent.posthog,
          [purpose]: !cookieConsent.posthog[purpose],
        },
      },
      `identity-web:account-privacy:posthog:${purpose}`,
    );
  };

  return (
    <div className="space-y-4">
      <div className="rounded-xl border border-border/40 bg-muted/20 p-4 space-y-2">
        <div className="flex items-center justify-between">
          <div>
            <p className="text-sm font-semibold text-foreground flex items-center gap-1.5">
              <span>🔒</span> Essentiels et Sécurité
            </p>
            <p className="text-xs text-muted-foreground mt-0.5">
              Nécessaires au fonctionnement technique et à la sécurité.
            </p>
          </div>
          <span className="text-[10px] font-bold uppercase tracking-wider text-emerald-600 bg-emerald-500/10 px-2 py-0.5 rounded">
            Obligatoire
          </span>
        </div>
        <div className="pl-5 pt-1.5 border-t border-border/20 flex flex-wrap gap-x-4 gap-y-1">
          <span className="text-xs text-muted-foreground">⚡ nvbes Identity</span>
          <span className="text-xs text-muted-foreground">💳 Stripe (Paiements et Fraude)</span>
        </div>
      </div>

      <div className="rounded-xl border border-border/40 p-4 space-y-3">
        <ConsentToggle
          checked={cookieConsent.categories.analytics}
          onChange={() => toggleCategory('analytics')}
          label="Analyses d'audience"
          description="Mesure l'utilisation pour l'amélioration continue."
          large
        />
        <div className="pl-5 pt-2 border-t border-border/20 space-y-2">
          <ConsentToggle
            checked={cookieConsent.vendors.posthog}
            onChange={() => toggleVendor('posthog')}
            label="PostHog"
            description="Active ou désactive toutes les finalités PostHog."
          />
          <ConsentToggle
            checked={cookieConsent.posthog.productAnalytics}
            onChange={() => togglePostHogPurpose('productAnalytics')}
            label="Analytics produit"
            description="Funnel produit pseudonymisé, sans emails, noms ou fichiers."
          />
          <ConsentToggle
            checked={cookieConsent.posthog.featureFlags}
            onChange={() => togglePostHogPurpose('featureFlags')}
            label="Feature flags"
            description="Expériences non critiques, jamais auth, sécurité ou billing."
          />
          <ConsentToggle
            checked={cookieConsent.posthog.autocaptureHeatmaps}
            onChange={() => togglePostHogPurpose('autocaptureHeatmaps')}
            label="Heatmaps & autocapture"
            description="Interactions masquées, désactivées sur les routes sensibles."
          />
          <ConsentToggle
            checked={cookieConsent.posthog.sessionReplay}
            onChange={() => togglePostHogPurpose('sessionReplay')}
            label="Session replay"
            description="Replay masqué, bloqué sur auth, MFA, privacy, billing et fichiers."
          />
          <ConsentToggle
            checked={cookieConsent.posthog.surveysFeedback}
            onChange={() => togglePostHogPurpose('surveysFeedback')}
            label="Surveys & feedback"
            description="Feedback post-auth, jamais sur les pages sensibles."
          />
        </div>
      </div>

      <div className="rounded-xl border border-border/40 p-4 space-y-3">
        <ConsentToggle
          checked={cookieConsent.categories.performance}
          onChange={() => toggleCategory('performance')}
          label="Performance & Erreurs"
          description="Suivi de la stabilité et diagnostics techniques."
          large
        />
        <div className="pl-5 pt-2 border-t border-border/20 space-y-2">
          <ConsentToggle
            checked={cookieConsent.vendors.sentry}
            onChange={() => toggleVendor('sentry')}
            label="Sentry"
            description="Rapports d'erreurs en temps réel."
          />
          <ConsentToggle
            checked={cookieConsent.posthog.errorTracking}
            onChange={() => togglePostHogPurpose('errorTracking')}
            label="PostHog error tracking"
            description="Capture navigateur scrubbed en parallèle de Sentry."
          />
        </div>
      </div>
    </div>
  );
}
