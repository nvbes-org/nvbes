import { type ReactNode, useState } from 'react';
import { TrackingConsentToggle, type TrackingConsentToggleProps } from './TrackingConsentToggle';
import {
  ACCEPT_ALL_CONSENT,
  type CookieConsentState,
  DECLINE_ALL_CONSENT,
  DEFAULT_CONSENT,
} from './tracking-consent';
import { toggleConsentAnalyticsPurpose, toggleConsentCategory } from './tracking-consent.editor';

export type TrackingConsentToggleComponent = (props: TrackingConsentToggleProps) => ReactNode;

const analyticsPurposeChoices = [
  {
    key: 'productAnalytics',
    label: "Mesure d'audience produit",
    description: 'Pages et fonctions utilisées, sans contenu utilisateur.',
  },
] as const;

export function SharedTrackingConsentBanner({
  getTrackingConsent,
  setTrackingConsent,
  sourcePrefix,
  ToggleComponent = TrackingConsentToggle,
}: {
  getTrackingConsent: () => CookieConsentState | null;
  setTrackingConsent: (consent: CookieConsentState, source?: string) => void;
  sourcePrefix: string;
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
    return null;
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
          <p className="text-xs leading-relaxed text-foreground/80">
            Nous respectons votre vie privée. nvbes et ses partenaires utilisent des cookies pour
            faire fonctionner la plateforme, mesurer l&apos;audience et analyser les performances
            techniques. Le refus désactive ces usages optionnels sans bloquer le service. Si vous
            êtes connecté, votre choix est synchronisé avec votre compte.
          </p>
          <a
            href="/legal/privacy-policy"
            className="inline-flex text-xs font-medium text-primary hover:underline"
          >
            Politique de confidentialité et informations sur les traceurs
          </a>
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
          <div className="space-y-5 border-t border-border/60 pt-5">
            <div className="max-w-md space-y-1.5">
              <p className="text-xs font-semibold uppercase tracking-[0.14em] text-muted-foreground">
                Préférences de confidentialité
              </p>
              <p className="text-xs leading-relaxed text-foreground/80">
                Activez uniquement les finalités utiles. Les cookies essentiels restent nécessaires
                à la sécurité du service.
              </p>
            </div>

            <div className="max-h-[40vh] space-y-3 overflow-y-auto pr-1">
              <section className="rounded-xl border border-border/60 bg-muted/20 px-4 py-3.5">
                <div className="flex items-start justify-between gap-4">
                  <div className="space-y-0.5">
                    <p className="text-sm font-semibold text-foreground">Essentiels et Sécurité</p>
                    <p className="text-xs leading-relaxed text-foreground/80">
                      Nécessaires au fonctionnement et à la sécurité.
                    </p>
                  </div>
                  <span className="shrink-0 rounded-md bg-emerald-500/10 px-2 py-1 text-[10px] font-bold uppercase tracking-[0.1em] text-emerald-700 dark:text-emerald-400">
                    Obligatoire
                  </span>
                </div>
                <div className="mt-3 flex flex-wrap gap-x-4 gap-y-1.5 border-t border-border/40 pt-3">
                  <span className="text-[11px] text-muted-foreground">nvbes Identity</span>
                  <span className="text-[11px] text-muted-foreground">
                    Stripe (Paiement/Fraude)
                  </span>
                  <span className="text-[11px] text-muted-foreground">Cloudflare (WAF/CDN)</span>
                </div>
              </section>

              <section className="divide-y divide-border/50 rounded-xl border border-border/60 bg-background">
                <ToggleComponent
                  checked={tempConsent.categories.analytics}
                  onChange={() =>
                    setTempConsent((prev) => toggleConsentCategory(prev, 'analytics'))
                  }
                  label="Analyse d'audience"
                  description="Mesure l'utilisation pour améliorer le service."
                  large
                />
                <div className="space-y-3 px-4 py-3.5">
                  {analyticsPurposeChoices.map((purpose) => (
                    <ToggleComponent
                      key={purpose.key}
                      checked={tempConsent.analytics[purpose.key]}
                      onChange={() =>
                        setTempConsent((previous) =>
                          toggleConsentAnalyticsPurpose(previous, purpose.key),
                        )
                      }
                      label={purpose.label}
                      description={purpose.description}
                    />
                  ))}
                  <p className="text-[11px] text-muted-foreground">
                    Fournisseur de ces finalités : PostHog.
                  </p>
                </div>
              </section>

              <section className="divide-y divide-border/50 rounded-xl border border-border/60 bg-background">
                <ToggleComponent
                  checked={tempConsent.categories.performance}
                  onChange={() =>
                    setTempConsent((prev) => toggleConsentCategory(prev, 'performance'))
                  }
                  label="Performance & erreurs"
                  description="Suivi de la stabilité et des bugs techniques."
                  large
                />
                <div className="space-y-3 px-4 py-3.5">
                  <ToggleComponent
                    checked={tempConsent.analytics.errorTracking}
                    onChange={() =>
                      setTempConsent((previous) =>
                        toggleConsentAnalyticsPurpose(previous, 'errorTracking'),
                      )
                    }
                    label="Rapports d'erreurs navigateur"
                    description="Diagnostics applicatifs minimisés, hors pages sensibles."
                  />
                  <p className="text-[11px] text-muted-foreground">
                    Fournisseurs concernés : Sentry et, selon le déploiement, Grafana Labs.
                  </p>
                </div>
              </section>
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
