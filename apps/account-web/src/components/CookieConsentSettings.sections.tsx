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

function CookieConsentToggleSection({
  title,
  description,
  id,
  label,
  settingDescription,
  checked,
  onCheckedChange,
}: {
  title: string;
  description: string;
  id: string;
  label: string;
  settingDescription: string;
  checked: boolean;
  onCheckedChange: () => void;
}) {
  return (
    <>
      <FieldSeparator />
      <FieldSet>
        <FieldLegend>{title}</FieldLegend>
        <FieldDescription>{description}</FieldDescription>
        <FieldGroup>
          <Field orientation="horizontal">
            <FieldContent>
              <FieldLabel htmlFor={id}>{label}</FieldLabel>
              <FieldDescription>{settingDescription}</FieldDescription>
            </FieldContent>
            <Switch id={id} checked={checked} onCheckedChange={onCheckedChange} />
          </Field>
        </FieldGroup>
      </FieldSet>
    </>
  );
}

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
    <CookieConsentToggleSection
      title="Comprendre l'usage du produit"
      description="Nous aide à savoir quelles fonctions sont utiles, sans lire votre contenu. Fourni par PostHog."
      id="privacy-product-analytics"
      label="Mesure d'audience"
      settingDescription="Pages et fonctionnalités utilisées, sans contenu utilisateur."
      checked={cookieConsent.analytics.productAnalytics}
      onCheckedChange={() => onTogglePurpose('productAnalytics')}
    />
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
    <CookieConsentToggleSection
      title="Corriger les problèmes techniques"
      description="Autorise des diagnostics minimisés pour améliorer la stabilité. Fourni par Sentry et, selon le déploiement, Grafana Labs."
      id="privacy-error-tracking"
      label="Rapports d'erreurs"
      settingDescription="Informations techniques sur les pannes, hors pages sensibles."
      checked={cookieConsent.analytics.errorTracking}
      onCheckedChange={() => onTogglePurpose('errorTracking')}
    />
  );
}
