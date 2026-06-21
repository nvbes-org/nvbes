import { useState } from 'react';
import {
  ACCEPT_ALL_CONSENT,
  DEFAULT_CONSENT,
  DECLINE_ALL_CONSENT,
  type CookieConsentState,
} from './tracking-consent';
import {
  toggleConsentCategory,
  toggleConsentAnalyticsPurpose,
  toggleConsentVendor,
} from './tracking-consent.editor';
import { TrackingConsentToggle } from './TrackingConsentToggle';

export function SharedTrackingConsentBanner({
  getTrackingConsent,
  setTrackingConsent,
  sourcePrefix,
  sessionReplayDescription,
}: {
  getTrackingConsent: () => CookieConsentState | null;
  setTrackingConsent: (consent: CookieConsentState, source?: string) => void;
  sourcePrefix: string;
  sessionReplayDescription: string;
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
        className="fixed bottom-4 right-4 z-50 rounded-full border border-border bg-background/80 px-4 py-2 text-xs font-semibold shadow-lg shadow-black/5 backdrop-blur transition-all duration-300 hover:scale-105 hover:bg-background"
        style={{ animation: 'pulse 2s infinite' }}
      >
        🍪 Préférences Cookies
      </button>
    );
  }

  return (
    <div className="fixed inset-x-4 bottom-4 z-50 max-h-[85vh] overflow-y-auto rounded-2xl border border-border/80 bg-background/95 p-5 shadow-2xl shadow-black/20 backdrop-blur-md transition-all duration-300 sm:inset-x-auto sm:right-4 sm:w-[32rem]">
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
          <div className="animate-fade-slide-up space-y-4 border-t border-border/60 pt-4">
            <p className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
              Préférences de confidentialité
            </p>

            <div className="max-h-[40vh] space-y-3.5 overflow-y-auto pr-1">
              <div className="space-y-2 rounded-xl border border-border/40 bg-muted/20 p-3">
                <div className="flex items-center justify-between">
                  <div>
                    <p className="flex items-center gap-1.5 text-xs font-semibold text-foreground">
                      <span>🔒</span> Essentiels et Sécurité
                    </p>
                    <p className="mt-0.5 text-[11px] text-muted-foreground">
                      Nécessaires au fonctionnement et à la sécurité.
                    </p>
                  </div>
                  <span className="rounded bg-emerald-500/10 px-2 py-0.5 text-[10px] font-bold uppercase tracking-wider text-emerald-600">
                    Obligatoire
                  </span>
                </div>
                <div className="flex flex-wrap gap-x-4 gap-y-1 border-t border-border/20 pl-5 pt-1">
                  <span className="text-[10px] text-muted-foreground">⚡ nvbes Identity</span>
                  <span className="text-[10px] text-muted-foreground">
                    💳 Stripe (Paiement/Fraude)
                  </span>
                </div>
              </div>

              <div className="space-y-2 rounded-xl border border-border/40 p-3">
                <div className="flex items-center justify-between">
                  <div>
                    <p className="flex items-center gap-1.5 text-xs font-semibold text-foreground">
                      <span>📈</span> Analyse d&apos;audience
                    </p>
                    <p className="mt-0.5 text-[11px] text-muted-foreground">
                      Mesure l&apos;utilisation pour améliorer le service.
                    </p>
                  </div>
                  <label className="relative inline-flex cursor-pointer items-center">
                    <input
                      type="checkbox"
                      checked={tempConsent.categories.analytics}
                      onChange={() =>
                        setTempConsent((prev) => toggleConsentCategory(prev, 'analytics'))
                      }
                      className="sr-only peer"
                    />
                    <div className="h-4 w-8 rounded-full bg-muted peer peer-checked:bg-primary peer-focus:outline-none peer-checked:after:translate-x-full peer-checked:after:border-white after:absolute after:left-[2px] after:top-[2px] after:h-3 after:w-3 after:rounded-full after:border after:border-border after:bg-background after:transition-all after:content-['']" />
                  </label>
                </div>
                <div className="space-y-2 border-t border-border/20 pl-5 pt-2">
                  <TrackingConsentToggle
                    checked={tempConsent.vendors.analytics}
                    onChange={() =>
                      setTempConsent((prev) => toggleConsentVendor(prev, 'analytics', 'analytics'))
                    }
                    label="Analytics"
                    description="Active ou désactive toutes les finalités Analytics."
                  />
                  <TrackingConsentToggle
                    checked={tempConsent.analytics.productAnalytics}
                    onChange={() =>
                      setTempConsent((prev) =>
                        toggleConsentAnalyticsPurpose(prev, 'productAnalytics'),
                      )
                    }
                    label="Analytics produit"
                    description="Mesure les étapes de funnel sans données personnelles."
                  />
                  <TrackingConsentToggle
                    checked={tempConsent.analytics.featureFlags}
                    onChange={() =>
                      setTempConsent((prev) => toggleConsentAnalyticsPurpose(prev, 'featureFlags'))
                    }
                    label="Feature flags"
                    description="Active des expériences non critiques après consentement."
                  />
                  <TrackingConsentToggle
                    checked={tempConsent.analytics.autocaptureHeatmaps}
                    onChange={() =>
                      setTempConsent((prev) =>
                        toggleConsentAnalyticsPurpose(prev, 'autocaptureHeatmaps'),
                      )
                    }
                    label="Heatmaps & autocapture"
                    description="Capture uniquement les interactions masquées et non sensibles."
                  />
                  <TrackingConsentToggle
                    checked={tempConsent.analytics.sessionReplay}
                    onChange={() =>
                      setTempConsent((prev) => toggleConsentAnalyticsPurpose(prev, 'sessionReplay'))
                    }
                    label="Session replay"
                    description={sessionReplayDescription}
                  />
                  <TrackingConsentToggle
                    checked={tempConsent.analytics.surveysFeedback}
                    onChange={() =>
                      setTempConsent((prev) =>
                        toggleConsentAnalyticsPurpose(prev, 'surveysFeedback'),
                      )
                    }
                    label="Surveys & feedback"
                    description="Questionnaires ciblés hors pages sensibles."
                  />
                </div>
              </div>

              <div className="space-y-2 rounded-xl border border-border/40 p-3">
                <div className="flex items-center justify-between">
                  <div>
                    <p className="flex items-center gap-1.5 text-xs font-semibold text-foreground">
                      <span>🛠️</span> Performance & Erreurs
                    </p>
                    <p className="mt-0.5 text-[11px] text-muted-foreground">
                      Suivi de la stabilité et des bugs techniques.
                    </p>
                  </div>
                  <label className="relative inline-flex cursor-pointer items-center">
                    <input
                      type="checkbox"
                      checked={tempConsent.categories.performance}
                      onChange={() =>
                        setTempConsent((prev) => toggleConsentCategory(prev, 'performance'))
                      }
                      className="sr-only peer"
                    />
                    <div className="h-4 w-8 rounded-full bg-muted peer peer-checked:bg-primary peer-focus:outline-none peer-checked:after:translate-x-full peer-checked:after:border-white after:absolute after:left-[2px] after:top-[2px] after:h-3 after:w-3 after:rounded-full after:border after:border-border after:bg-background after:transition-all after:content-['']" />
                  </label>
                </div>
                <div className="space-y-2 border-t border-border/20 pl-5 pt-2">
                  <TrackingConsentToggle
                    checked={tempConsent.vendors.errorReporting}
                    onChange={() =>
                      setTempConsent((prev) =>
                        toggleConsentVendor(prev, 'errorReporting', 'performance'),
                      )
                    }
                    label="Error reporting"
                    description="Rapports d'erreurs techniques."
                  />
                  <TrackingConsentToggle
                    checked={tempConsent.analytics.errorTracking}
                    onChange={() =>
                      setTempConsent((prev) => toggleConsentAnalyticsPurpose(prev, 'errorTracking'))
                    }
                    label="Analytics error tracking"
                    description="Capture navigateur scrubbed pour les diagnostics produit."
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
