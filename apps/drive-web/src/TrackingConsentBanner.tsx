import { useState } from 'react';
import {
  getTrackingConsent,
  setTrackingConsent,
  ACCEPT_ALL_CONSENT,
  DECLINE_ALL_CONSENT,
  DEFAULT_CONSENT,
  type CookieConsentState,
} from './tracking-consent';

export function TrackingConsentBanner() {
  const currentConsent = getTrackingConsent();
  const hasChoice = currentConsent !== null;
  const [open, setOpen] = useState(() => !hasChoice);
  const [showCustomize, setShowCustomize] = useState(false);

  // Initialize custom state with current consent or default
  const [tempConsent, setTempConsent] = useState<CookieConsentState>(
    () => currentConsent || DEFAULT_CONSENT,
  );

  if (!open) {
    return (
      <button
        type="button"
        onClick={() => setOpen(true)}
        className="fixed bottom-4 right-4 z-50 rounded-full border border-border bg-background/80 hover:bg-background px-4 py-2 text-xs font-semibold shadow-lg shadow-black/5 backdrop-blur transition-all duration-300 hover:scale-105"
        style={{ animation: 'pulse 2s infinite' }}
      >
        🍪 Préférences Cookies
      </button>
    );
  }

  const handleAcceptAll = () => {
    setTrackingConsent(ACCEPT_ALL_CONSENT, 'drive-web:banner:accept-all');
    window.location.reload();
  };

  const handleDeclineAll = () => {
    setTrackingConsent(DECLINE_ALL_CONSENT, 'drive-web:banner:decline-all');
    window.location.reload();
  };

  const handleSaveCustom = () => {
    setTrackingConsent(tempConsent, 'drive-web:banner:custom');
    window.location.reload();
  };

  const toggleCategory = (category: keyof CookieConsentState['categories']) => {
    if (category === 'essentials') return;

    setTempConsent((prev) => {
      const nextVal = !prev.categories[category];
      const nextCategories = { ...prev.categories, [category]: nextVal };
      const nextVendors = { ...prev.vendors };

      // Update vendors inside this category
      if (category === 'analytics') {
        nextVendors.posthog = nextVal;
      } else if (category === 'performance') {
        nextVendors.sentry = nextVal;
      }

      return {
        categories: nextCategories,
        vendors: nextVendors,
      };
    });
  };

  const toggleVendor = (
    vendor: keyof CookieConsentState['vendors'],
    category: keyof CookieConsentState['categories'],
  ) => {
    if (vendor === 'stripe' || vendor === 'identity') return;

    setTempConsent((prev) => {
      const nextVal = !prev.vendors[vendor];
      const nextVendors = { ...prev.vendors, [vendor]: nextVal };

      // Update category if vendor changes
      const nextCategories = { ...prev.categories };
      if (category === 'analytics') {
        nextCategories.analytics = nextVendors.posthog;
      } else if (category === 'performance') {
        nextCategories.performance = nextVendors.sentry;
      }

      return {
        categories: nextCategories,
        vendors: nextVendors,
      };
    });
  };

  return (
    <div className="fixed inset-x-4 bottom-4 z-50 rounded-2xl border border-border/80 bg-background/95 p-5 shadow-2xl shadow-black/20 backdrop-blur-md transition-all duration-300 sm:inset-x-auto sm:right-4 sm:w-[32rem] max-h-[85vh] overflow-y-auto">
      <div className="space-y-5">
        <div className="space-y-2">
          <div className="flex items-center gap-2">
            <span className="text-xl">🍪</span>
            <p className="text-base font-semibold tracking-tight text-foreground">
              Gestion des cookies & traceurs
            </p>
          </div>
          <p className="text-xs text-muted-foreground leading-relaxed">
            Nous respectons votre vie privée. nvbes et ses partenaires utilisent des cookies pour
            faire fonctionner la plateforme, mesurer l'audience et analyser les performances
            techniques.
          </p>
        </div>

        {!showCustomize ? (
          <div className="flex flex-col gap-2">
            <div className="flex flex-wrap gap-2 pt-2">
              <button
                type="button"
                onClick={handleDeclineAll}
                className="flex-1 min-w-[100px] rounded-full border border-border bg-transparent px-4 py-2.5 text-xs font-semibold hover:bg-muted text-foreground transition-all"
              >
                Tout refuser
              </button>
              <button
                type="button"
                onClick={() => setShowCustomize(true)}
                className="flex-1 min-w-[100px] rounded-full border border-border bg-transparent px-4 py-2.5 text-xs font-semibold hover:bg-muted text-foreground transition-all"
              >
                Personnaliser
              </button>
              <button
                type="button"
                onClick={handleAcceptAll}
                className="flex-1 min-w-[100px] rounded-full bg-primary px-4 py-2.5 text-xs font-semibold text-primary-foreground hover:opacity-90 transition-all shadow-sm"
              >
                Tout accepter
              </button>
            </div>
            {hasChoice && (
              <button
                type="button"
                onClick={() => setOpen(false)}
                className="w-full text-center text-xs text-muted-foreground hover:text-foreground py-1 transition-all"
              >
                Fermer sans modifier
              </button>
            )}
          </div>
        ) : (
          <div className="space-y-4 border-t border-border/60 pt-4 animate-fade-slide-up">
            <p className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
              Préférences de confidentialité
            </p>

            <div className="space-y-3.5 max-h-[40vh] overflow-y-auto pr-1">
              {/* Category: Essentials */}
              <div className="rounded-xl border border-border/40 bg-muted/20 p-3 space-y-2">
                <div className="flex items-center justify-between">
                  <div>
                    <p className="text-xs font-semibold text-foreground flex items-center gap-1.5">
                      <span>🔒</span> Essentiels et Sécurité
                    </p>
                    <p className="text-[11px] text-muted-foreground mt-0.5">
                      Nécessaires au fonctionnement et à la sécurité.
                    </p>
                  </div>
                  <span className="text-[10px] font-bold uppercase tracking-wider text-emerald-600 bg-emerald-500/10 px-2 py-0.5 rounded">
                    Obligatoire
                  </span>
                </div>
                <div className="pl-5 pt-1 border-t border-border/20 flex flex-wrap gap-x-4 gap-y-1">
                  <span className="text-[10px] text-muted-foreground">⚡ nvbes Identity</span>
                  <span className="text-[10px] text-muted-foreground">
                    💳 Stripe (Paiement/Fraude)
                  </span>
                </div>
              </div>

              {/* Category: Analytics */}
              <div className="rounded-xl border border-border/40 p-3 space-y-2">
                <div className="flex items-center justify-between">
                  <div>
                    <p className="text-xs font-semibold text-foreground flex items-center gap-1.5">
                      <span>📈</span> Analyse d'audience
                    </p>
                    <p className="text-[11px] text-muted-foreground mt-0.5">
                      Mesure l'utilisation pour améliorer le service.
                    </p>
                  </div>
                  <label className="relative inline-flex items-center cursor-pointer">
                    <input
                      type="checkbox"
                      checked={tempConsent.categories.analytics}
                      onChange={() => toggleCategory('analytics')}
                      className="sr-only peer"
                    />
                    <div className="w-8 h-4 bg-muted peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-background after:border-border after:border after:rounded-full after:h-3 after:w-3 after:transition-all peer-checked:bg-primary" />
                  </label>
                </div>
                <div className="pl-5 pt-2 border-t border-border/20 space-y-1.5">
                  <div className="flex items-center justify-between">
                    <div>
                      <p className="text-[11px] font-medium text-foreground">PostHog</p>
                      <p className="text-[10px] text-muted-foreground">
                        Analyses comportementales et de parcours.
                      </p>
                    </div>
                    <label className="relative inline-flex items-center cursor-pointer">
                      <input
                        type="checkbox"
                        checked={tempConsent.vendors.posthog}
                        onChange={() => toggleVendor('posthog', 'analytics')}
                        className="sr-only peer"
                      />
                      <div className="w-7 h-3.5 bg-muted peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-background after:border-border after:border after:rounded-full after:h-2.5 after:w-2.5 after:transition-all peer-checked:bg-primary" />
                    </label>
                  </div>
                </div>
              </div>

              {/* Category: Performance */}
              <div className="rounded-xl border border-border/40 p-3 space-y-2">
                <div className="flex items-center justify-between">
                  <div>
                    <p className="text-xs font-semibold text-foreground flex items-center gap-1.5">
                      <span>🛠️</span> Performance & Erreurs
                    </p>
                    <p className="text-[11px] text-muted-foreground mt-0.5">
                      Suivi de la stabilité et des bugs techniques.
                    </p>
                  </div>
                  <label className="relative inline-flex items-center cursor-pointer">
                    <input
                      type="checkbox"
                      checked={tempConsent.categories.performance}
                      onChange={() => toggleCategory('performance')}
                      className="sr-only peer"
                    />
                    <div className="w-8 h-4 bg-muted peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-background after:border-border after:border after:rounded-full after:h-3 after:w-3 after:transition-all peer-checked:bg-primary" />
                  </label>
                </div>
                <div className="pl-5 pt-2 border-t border-border/20 space-y-1.5">
                  <div className="flex items-center justify-between">
                    <div>
                      <p className="text-[11px] font-medium text-foreground">Sentry</p>
                      <p className="text-[10px] text-muted-foreground">
                        Rapports d'erreurs en temps réel.
                      </p>
                    </div>
                    <label className="relative inline-flex items-center cursor-pointer">
                      <input
                        type="checkbox"
                        checked={tempConsent.vendors.sentry}
                        onChange={() => toggleVendor('sentry', 'performance')}
                        className="sr-only peer"
                      />
                      <div className="w-7 h-3.5 bg-muted peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-background after:border-border after:border after:rounded-full after:h-2.5 after:w-2.5 after:transition-all peer-checked:bg-primary" />
                    </label>
                  </div>
                </div>
              </div>
            </div>

            <div className="flex gap-2 pt-2">
              <button
                type="button"
                onClick={() => setShowCustomize(false)}
                className="rounded-full border border-border bg-transparent px-4 py-2 text-xs font-semibold text-foreground hover:bg-muted transition-all"
              >
                Retour
              </button>
              <button
                type="button"
                onClick={handleSaveCustom}
                className="flex-1 rounded-full bg-primary px-4 py-2 text-xs font-semibold text-primary-foreground hover:opacity-90 transition-all shadow-sm"
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
