import { TrackingConsentToggle } from '@nvbes/web-runtime';
import { Badge } from '@/components/ui/badge';
import type { CookieConsentState } from '../tracking-consent';
import { essentialVendors, posthogPurposes } from './CookieConsentSettings.shared';

export function CookieConsentEssentialSection() {
  return (
    <div className="space-y-2 p-4">
      <div className="flex items-center justify-between">
        <div>
          <p className="flex items-center gap-1.5 font-semibold text-foreground">
            <span>🔒</span> Essentiels et Sécurité
          </p>
          <p className="mt-0.5 text-muted-foreground">
            Nécessaires au fonctionnement technique et à la sécurité.
          </p>
        </div>
        <Badge variant="secondary" className="uppercase tracking-wider">
          Obligatoire
        </Badge>
      </div>
      <div className="flex flex-wrap gap-x-4 gap-y-1 border-t border-border/20">
        {essentialVendors.map((vendor) => (
          <span key={vendor} className="text-muted-foreground">
            {vendor}
          </span>
        ))}
      </div>
    </div>
  );
}

export function CookieConsentAnalyticsSection({
  cookieConsent,
  onToggleCategory,
  onToggleVendor,
  onTogglePostHogPurpose,
}: {
  cookieConsent: CookieConsentState;
  onToggleCategory: (category: keyof CookieConsentState['categories']) => void;
  onToggleVendor: (vendor: keyof CookieConsentState['vendors']) => void;
  onTogglePostHogPurpose: (purpose: keyof CookieConsentState['posthog']) => void;
}) {
  return (
    <div className="space-y-3 p-4">
      <TrackingConsentToggle
        checked={cookieConsent.categories.analytics}
        onChange={() => onToggleCategory('analytics')}
        label="Analyses d'audience"
        description="Mesure l'utilisation pour l'amélioration continue."
        large
      />
      <div className="space-y-2 border-t border-border/20 px-2">
        <TrackingConsentToggle
          checked={cookieConsent.vendors.posthog}
          onChange={() => onToggleVendor('posthog')}
          label="PostHog"
          description="Active ou désactive toutes les finalités PostHog."
        />
        {posthogPurposes
          .filter((purpose) => purpose.key !== 'errorTracking')
          .map((purpose) => (
            <TrackingConsentToggle
              key={purpose.key}
              checked={cookieConsent.posthog[purpose.key]}
              onChange={() => onTogglePostHogPurpose(purpose.key)}
              label={purpose.label}
              description={purpose.description}
            />
          ))}
      </div>
    </div>
  );
}

export function CookieConsentPerformanceSection({
  cookieConsent,
  onToggleCategory,
  onToggleVendor,
  onTogglePostHogPurpose,
}: {
  cookieConsent: CookieConsentState;
  onToggleCategory: (category: keyof CookieConsentState['categories']) => void;
  onToggleVendor: (vendor: keyof CookieConsentState['vendors']) => void;
  onTogglePostHogPurpose: (purpose: keyof CookieConsentState['posthog']) => void;
}) {
  const errorTracking = posthogPurposes.find((purpose) => purpose.key === 'errorTracking');

  return (
    <div className="space-y-3 p-4">
      <TrackingConsentToggle
        checked={cookieConsent.categories.performance}
        onChange={() => onToggleCategory('performance')}
        label="Performance & Erreurs"
        description="Suivi de la stabilité et diagnostics techniques."
        large
      />
      <div className="space-y-2 border-t border-border/20 px-2">
        <TrackingConsentToggle
          checked={cookieConsent.vendors.sentry}
          onChange={() => onToggleVendor('sentry')}
          label="Sentry"
          description="Rapports d'erreurs en temps réel."
        />
        {errorTracking ? (
          <TrackingConsentToggle
            checked={cookieConsent.posthog.errorTracking}
            onChange={() => onTogglePostHogPurpose('errorTracking')}
            label={errorTracking.label}
            description={errorTracking.description}
          />
        ) : null}
      </div>
    </div>
  );
}
