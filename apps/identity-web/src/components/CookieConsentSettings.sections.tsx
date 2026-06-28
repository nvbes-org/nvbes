import { Badge } from '@/components/ui/badge';
import type { CookieConsentState } from '../tracking-consent';
import {
  analyticsVendors,
  essentialVendors,
  performanceVendors,
} from './CookieConsentSettings.shared';
import { TrackingConsentToggle } from './TrackingConsentToggle';

export function CookieConsentEssentialSection() {
  return (
    <div className="space-y-2 p-4">
      <div className="flex items-center justify-between">
        <div>
          <p className="font-semibold text-foreground">Essentiels et Sécurité</p>
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
}: {
  cookieConsent: CookieConsentState;
  onToggleCategory: (category: keyof CookieConsentState['categories']) => void;
  onToggleVendor: (vendor: keyof CookieConsentState['vendors']) => void;
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
        {analyticsVendors.map((vendor) => (
          <TrackingConsentToggle
            key={vendor.key}
            checked={cookieConsent.vendors[vendor.key]}
            onChange={() => onToggleVendor(vendor.key)}
            label={vendor.label}
            description={vendor.description}
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
}: {
  cookieConsent: CookieConsentState;
  onToggleCategory: (category: keyof CookieConsentState['categories']) => void;
  onToggleVendor: (vendor: keyof CookieConsentState['vendors']) => void;
}) {
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
        {performanceVendors.map((vendor) => (
          <TrackingConsentToggle
            key={vendor.key}
            checked={cookieConsent.vendors[vendor.key]}
            onChange={() => onToggleVendor(vendor.key)}
            label={vendor.label}
            description={vendor.description}
          />
        ))}
      </div>
    </div>
  );
}
