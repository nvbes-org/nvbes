import { Badge } from '@/components/ui/badge';
import {
  Field,
  FieldContent,
  FieldDescription,
  FieldGroup,
  FieldLabel,
  FieldLegend,
  FieldSeparator,
  FieldSet,
  FieldTitle,
} from '@/components/ui/field';
import { Switch } from '@/components/ui/switch';
import type { CookieConsentState } from '../tracking-consent';
import { essentialVendors } from './CookieConsentSettings.shared';

export function CookieConsentEssentialSection() {
  return (
    <FieldSet>
      <FieldLegend>Indispensables au service</FieldLegend>
      <FieldDescription>
        Ces éléments assurent la connexion, la sécurité et la continuité du service. Ils ne peuvent
        pas être désactivés.
      </FieldDescription>
      <FieldGroup>
        <Field orientation="horizontal" data-disabled="true">
          <FieldContent>
            <FieldTitle>Fonctionnement et sécurité</FieldTitle>
            <FieldDescription>{essentialVendors.join(' · ')}</FieldDescription>
          </FieldContent>
          <Badge variant="secondary">Toujours actif</Badge>
        </Field>
      </FieldGroup>
    </FieldSet>
  );
}

export function CookieConsentAnalyticsSection({
  cookieConsent,
  onTogglePurpose,
}: {
  cookieConsent: CookieConsentState;
  onTogglePurpose: (purpose: keyof CookieConsentState['analytics']) => void;
}) {
  return (
    <>
      <FieldSeparator />
      <FieldSet>
        <FieldLegend>Comprendre l'usage du produit</FieldLegend>
        <FieldDescription>
          Nous aide à savoir quelles fonctions sont utiles, sans lire votre contenu. Fourni par
          PostHog.
        </FieldDescription>
        <FieldGroup>
          <Field orientation="horizontal">
            <FieldContent>
              <FieldLabel htmlFor="privacy-product-analytics">Mesure d'audience</FieldLabel>
              <FieldDescription>
                Pages et fonctionnalités utilisées, sans contenu utilisateur.
              </FieldDescription>
            </FieldContent>
            <Switch
              id="privacy-product-analytics"
              checked={cookieConsent.analytics.productAnalytics}
              onCheckedChange={() => onTogglePurpose('productAnalytics')}
            />
          </Field>
        </FieldGroup>
      </FieldSet>
    </>
  );
}

export function CookieConsentPerformanceSection({
  cookieConsent,
  onTogglePurpose,
}: {
  cookieConsent: CookieConsentState;
  onTogglePurpose: (purpose: keyof CookieConsentState['analytics']) => void;
}) {
  return (
    <>
      <FieldSeparator />
      <FieldSet>
        <FieldLegend>Corriger les problèmes techniques</FieldLegend>
        <FieldDescription>
          Autorise des diagnostics minimisés pour améliorer la stabilité. Fourni par Sentry et,
          selon le déploiement, Grafana Labs.
        </FieldDescription>
        <FieldGroup>
          <Field orientation="horizontal">
            <FieldContent>
              <FieldLabel htmlFor="privacy-error-tracking">Rapports d'erreurs</FieldLabel>
              <FieldDescription>
                Informations techniques sur les pannes, hors pages sensibles.
              </FieldDescription>
            </FieldContent>
            <Switch
              id="privacy-error-tracking"
              checked={cookieConsent.analytics.errorTracking}
              onCheckedChange={() => onTogglePurpose('errorTracking')}
            />
          </Field>
        </FieldGroup>
      </FieldSet>
    </>
  );
}
