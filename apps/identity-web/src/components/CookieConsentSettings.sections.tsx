import { TrackingConsentToggle } from '@nvbes/web-runtime';
import { Badge } from '@/components/ui/badge';
import type { CookieConsentState } from '../tracking-consent';
import { essentialVendors, analyticsPurposes } from './CookieConsentSettings.shared';

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
  onToggleAnalyticsPurpose,
}: {
  cookieConsent: CookieConsentState;
  onToggleCategory: (category: keyof CookieConsentState['categories']) => void;
  onToggleVendor: (vendor: keyof CookieConsentState['vendors']) => void;
  onToggleAnalyticsPurpose: (purpose: keyof CookieConsentState['analytics']) => void;
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
          checked={cookieConsent.vendors.analytics}
          onChange={() => onToggleVendor('analytics')}
          label="Analytics"
          description="Active ou désactive toutes les finalités Analytics."
        />
        {analyticsPurposes
          .filter((purpose) => purpose.key !== 'errorTracking')
          .map((purpose) => (
            <TrackingConsentToggle
              key={purpose.key}
              checked={cookieConsent.analytics[purpose.key]}
              onChange={() => onToggleAnalyticsPurpose(purpose.key)}
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
  onToggleAnalyticsPurpose,
}: {
  cookieConsent: CookieConsentState;
  onToggleCategory: (category: keyof CookieConsentState['categories']) => void;
  onToggleVendor: (vendor: keyof CookieConsentState['vendors']) => void;
  onToggleAnalyticsPurpose: (purpose: keyof CookieConsentState['analytics']) => void;
}) {
  const errorTracking = analyticsPurposes.find((purpose) => purpose.key === 'errorTracking');

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
          checked={cookieConsent.vendors.errorReporting}
          onChange={() => onToggleVendor('errorReporting')}
          label="Error reporting"
          description="Rapports d'erreurs techniques."
        />
        {errorTracking ? (
          <TrackingConsentToggle
            checked={cookieConsent.analytics.errorTracking}
            onChange={() => onToggleAnalyticsPurpose('errorTracking')}
            label={errorTracking.label}
            description={errorTracking.description}
          />
        ) : null}
      </div>
    </div>
  );
}
