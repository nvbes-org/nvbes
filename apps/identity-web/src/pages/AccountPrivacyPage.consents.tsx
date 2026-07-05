import { type UserConsent } from '@nvbes/identity-client';
import { FileText, XCircle } from 'lucide-react';

import { Button } from '@/components/ui/button';

export const consentLabels: Record<string, string> = {
  terms_of_service: "Conditions d'utilisation",
  privacy_policy: 'Politique de confidentialite',
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
  data_processing: 'Traitement des donnees',
  third_party_sharing: 'Partage avec des tiers',
  gpc_opt_out: 'Global Privacy Control — Ne pas vendre mes donnees',
};

export function isVisibleConsentType(consentType: string): boolean {
  return !consentType.startsWith('analytics_');
}

export function ConsentEmptyState() {
  return (
    <div className="px-6 py-8">
      <div className="flex flex-col items-center gap-3">
        <XCircle className="size-8 text-muted-foreground" />
        <div className="text-center">
          <p className="text-sm text-muted-foreground">Aucun consentement actif.</p>
        </div>
      </div>
    </div>
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

  return (
    <div className="flex items-center justify-between gap-3 py-1">
      <div className="flex min-w-0 items-center gap-3">
        <div className="flex size-8 shrink-0 items-center justify-center rounded-lg bg-muted">
          <FileText className="size-4 text-muted-foreground" />
        </div>
        <div className="flex min-w-0 flex-col">
          <span className="text-sm font-medium">{label}</span>
          <span className="text-xs text-muted-foreground">
            Accepte le {grantedDate}
            {consent.document_version ? ` · v${consent.document_version}` : ''}
          </span>
        </div>
      </div>
      <Button
        variant="ghost"
        size="sm"
        className="h-8 w-8 p-0 text-muted-foreground hover:text-destructive"
        onClick={onRevoke}
        aria-label={`Revoquer ${label}`}
      >
        <XCircle className="size-4" />
      </Button>
    </div>
  );
}
