import { type ReactNode, useState } from 'react';
import {
  ACCEPT_ALL_CONSENT,
  DEFAULT_CONSENT,
  DECLINE_ALL_CONSENT,
  type CookieConsentState,
} from './tracking-consent';
import { toggleConsentCategory, toggleConsentVendor } from './tracking-consent.editor';
import { TrackingConsentToggle, type TrackingConsentToggleProps } from './TrackingConsentToggle';

export type TrackingConsentToggleComponent = (props: TrackingConsentToggleProps) => ReactNode;

export function SharedTrackingConsentBanner({
  getTrackingConsent,
  setTrackingConsent,
  sourcePrefix,
  sessionReplayDescription,
  ToggleComponent = TrackingConsentToggle,
}: {
  getTrackingConsent: () => CookieConsentState | null;
  setTrackingConsent: (consent: CookieConsentState, source?: string) => void;
  sourcePrefix: string;
  sessionReplayDescription: string;
  ToggleComponent?: TrackingConsentToggleComponent;
}) {
  const currentConsent = getTrackingConsent();
  const hasChoice = currentConsent !== null;
  const [open, setOpen] = useState(() => !hasChoice);
  const [showCustomize, setShowCustomize] = useState(false);
  const [tempConsent, setTempConsent] = useState<CookieConsentState>(
    () => currentConsent || DEFAULT_CONSENT,
  );

  if (!open) {
    return (
      <button
        type="button"
        onClick={() => setOpen(true)}
        className="fixed bottom-4 left-4 z-50 rounded-full border border-border bg-background/80 px-4 py-2 text-xs font-semibold shadow-lg shadow-black/5 backdrop-blur hover:bg-background"
      >
        🍪 Préférences Cookies
      </button>
    );
  }

  return (
    <div className="fixed inset-x-4 bottom-4 z-50 max-h-[85vh] overflow-y-auto rounded-2xl border border-border/80 bg-background/95 p-5 shadow-2xl shadow-black/20 backdrop-blur-md transition-all duration-300 sm:inset-x-auto sm:left-4 sm:w-[32rem]">
      <div className="space-y-5">
        <div className="space-y-2">
          <div className="flex items-center gap-2">
            <span className="text-xl">🍪</span>
            <p className="text-base font-semibold tracking-tight text-foreground">
              Gestion des cookies & traceurs
            </p>
          </div>
          <p className="text-xs leading-relaxed text-muted-foreground">
            Nous respectons votre vie privée. nvbes et ses partenaires utilisent des cookies pour
            faire fonctionner la plateforme, mesurer l&apos;audience et analyser les performances
            techniques.
          </p>
        </div>

        {!showCustomize ? (
          <div className="flex flex-col gap-2">
            <div className="flex flex-wrap gap-2 pt-2">
              <button
                type="button"
                onClick={() => {
                  setTrackingConsent(DECLINE_ALL_CONSENT, `${sourcePrefix}:banner:decline-all`);
                  window.location.reload();
                }}
                className="min-w-[100px] flex-1 rounded-full border border-border bg-transparent px-4 py-2.5 text-xs font-semibold text-foreground transition-all hover:bg-muted"
              >
                Tout refuser
              </button>
              <button
                type="button"
                onClick={() => setShowCustomize(true)}
                className="min-w-[100px] flex-1 rounded-full border border-border bg-transparent px-4 py-2.5 text-xs font-semibold text-foreground transition-all hover:bg-muted"
              >
                Personnaliser
              </button>
              <button
                type="button"
                onClick={() => {
                  setTrackingConsent(ACCEPT_ALL_CONSENT, `${sourcePrefix}:banner:accept-all`);
                  window.location.reload();
                }}
                className="min-w-[100px] flex-1 rounded-full bg-primary px-4 py-2.5 text-xs font-semibold text-primary-foreground shadow-sm transition-all hover:opacity-90"
              >
                Tout accepter
              </button>
            </div>
            {hasChoice && (
              <button
                type="button"
                onClick={() => setOpen(false)}
                className="w-full py-1 text-center text-xs text-muted-foreground transition-all hover:text-foreground"
              >
                Fermer sans modifier
              </button>
            )}
          </div>
        ) : (
          <div className="space-y-4 border-t border-border/60 pt-4">
            <div className="space-y-1">
              <p className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                Préférences de confidentialité
              </p>
              <p className="text-[11px] leading-relaxed text-muted-foreground">
                Activez uniquement les finalités utiles. Les cookies essentiels restent nécessaires
                à la sécurité du service.
              </p>
            </div>

            <div className="max-h-[40vh] space-y-3.5 overflow-y-auto pr-1">
              <div className="space-y-2 rounded-xl border border-border/40 bg-muted/20 p-3">
                <div className="flex items-center justify-between">
                  <div>
                    <p className="text-xs font-semibold text-foreground">Essentiels et Sécurité</p>
                    <p className="mt-0.5 text-[11px] text-muted-foreground">
                      Nécessaires au fonctionnement et à la sécurité.
                    </p>
                  </div>
                  <span className="rounded bg-emerald-500/10 px-2 py-0.5 text-[10px] font-bold uppercase tracking-wider text-emerald-600">
                    Obligatoire
                  </span>
                </div>
                <div className="flex flex-wrap gap-x-4 gap-y-1 border-t border-border/20 pl-5 pt-1">
                  <span className="text-[10px] text-muted-foreground">nvbes Identity</span>
                  <span className="text-[10px] text-muted-foreground">
                    Stripe (Paiement/Fraude)
                  </span>
                  <span className="text-[10px] text-muted-foreground">Cloudflare (WAF/CDN)</span>
                </div>
              </div>

              <div className="space-y-2 rounded-xl border border-border/40 bg-muted/10 p-3">
                <ToggleComponent
                  checked={tempConsent.categories.analytics}
                  onChange={() =>
                    setTempConsent((prev) => toggleConsentCategory(prev, 'analytics'))
                  }
                  label="Analyse d'audience"
                  description="Mesure l'utilisation pour améliorer le service."
                  large
                />
                <div className="space-y-2 border-t border-border/20 pl-5 pt-2">
                  <ToggleComponent
                    checked={tempConsent.vendors.posthog}
                    onChange={() =>
                      setTempConsent((prev) => toggleConsentVendor(prev, 'posthog', 'analytics'))
                    }
                    label="PostHog"
                    description={`Mesure d'audience produit, heatmaps, feature flags et ${sessionReplayDescription.toLowerCase()}`}
                  />
                </div>
              </div>

              <div className="space-y-2 rounded-xl border border-border/40 bg-muted/10 p-3">
                <ToggleComponent
                  checked={tempConsent.categories.performance}
                  onChange={() =>
                    setTempConsent((prev) => toggleConsentCategory(prev, 'performance'))
                  }
                  label="Performance & erreurs"
                  description="Suivi de la stabilité et des bugs techniques."
                  large
                />
                <div className="space-y-2 border-t border-border/20 pl-5 pt-2">
                  <ToggleComponent
                    checked={tempConsent.vendors.sentry}
                    onChange={() =>
                      setTempConsent((prev) => toggleConsentVendor(prev, 'sentry', 'performance'))
                    }
                    label="Sentry"
                    description="Suivi d'erreurs applicatives côté navigateur."
                  />
                  <ToggleComponent
                    checked={tempConsent.vendors.grafana}
                    onChange={() =>
                      setTempConsent((prev) => toggleConsentVendor(prev, 'grafana', 'performance'))
                    }
                    label="Grafana Labs"
                    description="Observabilité technique, métriques et diagnostics de stabilité."
                  />
                </div>
              </div>
            </div>

            <div className="flex gap-2 pt-2">
              <button
                type="button"
                onClick={() => setShowCustomize(false)}
                className="rounded-full border border-border bg-transparent px-4 py-2 text-xs font-semibold text-foreground transition-all hover:bg-muted"
              >
                Retour
              </button>
              <button
                type="button"
                onClick={() => {
                  setTrackingConsent(tempConsent, `${sourcePrefix}:banner:custom`);
                  window.location.reload();
                }}
                className="flex-1 rounded-full bg-primary px-4 py-2 text-xs font-semibold text-primary-foreground shadow-sm transition-all hover:opacity-90"
              >
                Enregistrer mes choix
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
