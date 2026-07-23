import type { UserConsent } from '@nvbes/identity-client';
import { FileCheck2, History, X } from 'lucide-react';

import { Button } from '@/components/ui/button';
import {
  Empty,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from '@/components/ui/empty';
import {
  Item,
  ItemActions,
  ItemContent,
  ItemDescription,
  ItemMedia,
  ItemTitle,
} from '@/components/ui/item';

export const consentLabels: Record<string, string> = {
  terms_of_service: "Conditions d'utilisation",
  data_processing_agreement: 'Accord sur le traitement des données',
  privacy_policy: 'Politique de confidentialité',
  cookie_consent: 'Consentement cookies global',
  cookie_consent_essentials: 'Cookies essentiels',
  cookie_consent_analytics: 'Cookies analytiques',
  cookie_consent_performance: 'Cookies de performance',
  cookie_consent_vendor_stripe: 'Cookie nvbes Billing',
  cookie_consent_vendor_identity: 'Cookie nvbes Identity',
  cookie_consent_vendor_cloudflare: 'Cookie Cloudflare',
  cookie_consent_vendor_posthog: 'Cookie PostHog',
  cookie_consent_vendor_sentry: 'Cookie Sentry',
  cookie_consent_vendor_grafana: 'Cookie Grafana Labs',
  cookie_consent_vendor_analytics: 'Cookie Analytics legacy',
  cookie_consent_vendor_error_reporting: 'Cookie error reporting legacy',
  marketing_emails: 'Emails marketing',
  data_processing: 'Traitement des données',
  third_party_sharing: 'Partage avec des tiers',
  gpc_opt_out: 'Global Privacy Control — Ne pas vendre mes donnees',
};

const NON_REVOCABLE_CONSENT_TYPES = new Set([
  'terms_of_service',
  'data_processing_agreement',
  'privacy_policy',
]);

export function isRevocableConsentType(consentType: string): boolean {
  return !NON_REVOCABLE_CONSENT_TYPES.has(consentType);
}

export function isVisibleConsentType(consentType: string): boolean {
  return !consentType.startsWith('analytics_');
}

export function ConsentEmptyState() {
  return (
    <Empty>
      <EmptyHeader>
        <EmptyMedia variant="icon">
          <History />
        </EmptyMedia>
        <EmptyTitle>Aucun choix révocable</EmptyTitle>
        <EmptyDescription>
          Vos futurs consentements apparaîtront ici avec leur date et leur version.
        </EmptyDescription>
      </EmptyHeader>
    </Empty>
  );
}

export function ConsentRow({
  consent,
  label,
  onRevoke,
}: {
  consent: UserConsent;
  label: string;
  onRevoke: () => void;
}) {
  const grantedDate = new Date(consent.granted_at).toLocaleDateString('fr-FR', {
    year: 'numeric',
    month: 'long',
    day: 'numeric',
  });
  const version = consent.document_version
    ? ` · ${consent.document_version.replace(/^v(?=\d)/i, 'version ')}`
    : '';

  return (
    <Item>
      <ItemMedia variant="icon">
        <FileCheck2 />
      </ItemMedia>
      <ItemContent>
        <ItemTitle>{label}</ItemTitle>
        <ItemDescription>
          Accepté le {grantedDate}
          {version}
        </ItemDescription>
      </ItemContent>
      {isRevocableConsentType(consent.consent_type) && (
        <ItemActions>
          <Button variant="ghost" size="icon" onClick={onRevoke} aria-label={`Révoquer ${label}`}>
            <X data-icon="inline-start" />
          </Button>
        </ItemActions>
      )}
    </Item>
  );
}
