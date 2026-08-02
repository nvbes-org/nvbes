import { toggleConsentCategory } from '@nvbes/web-runtime';
import { useMutation, useQuery } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import { accountClient } from '@/account.client';
import { useAccountAuthenticationRecovery } from '@/account.authentication';
import { accountQueryKeys } from '@/account.queries';
import { AccountPage } from '@/components/AccountPage';
import { AccountPageError, AccountPageLoading } from '@/components/AccountPageState';
import { TrackingConsentToggle } from '@/components/TrackingConsentToggle';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { AccountPrivacyExportActions } from '@/pages/AccountPrivacyPage.export';
import {
  DEFAULT_CONSENT,
  TRACKING_CONSENT_CHANGED_EVENT,
  getTrackingConsent,
  setTrackingConsent,
  type CookieConsentState,
} from '@/tracking-consent';

export default function AccountPrivacyPage() {
  const consentsQuery = useQuery({
    queryKey: accountQueryKeys.consents,
    queryFn: ({ signal }) => accountClient.listConsents({ signal, limit: 100 }),
  });
  const gpcQuery = useQuery({
    queryKey: accountQueryKeys.gpc,
    queryFn: ({ signal }) => accountClient.getGpcStatus({ signal }),
  });
  const [consent, setConsent] = useState<CookieConsentState>(
    () => getTrackingConsent() ?? DEFAULT_CONSENT,
  );

  useAccountAuthenticationRecovery(consentsQuery.error ?? gpcQuery.error);

  useEffect(() => {
    const syncLocalConsent = () => {
      setConsent(getTrackingConsent() ?? DEFAULT_CONSENT);
    };
    window.addEventListener(TRACKING_CONSENT_CHANGED_EVENT, syncLocalConsent);
    return () => window.removeEventListener(TRACKING_CONSENT_CHANGED_EVENT, syncLocalConsent);
  }, []);

  if (consentsQuery.isPending || gpcQuery.isPending) {
    return (
      <AccountPage
        title="Confidentialité"
        description="Contrôlez les traitements optionnels et vos données Account."
      >
        <AccountPageLoading label="Chargement de la confidentialité…" />
      </AccountPage>
    );
  }

  if (consentsQuery.error || gpcQuery.error || !consentsQuery.data || !gpcQuery.data) {
    return (
      <AccountPage
        title="Confidentialité"
        description="Contrôlez les traitements optionnels et vos données Account."
      >
        <AccountPageError
          error={consentsQuery.error ?? gpcQuery.error}
          onRetry={() => {
            void consentsQuery.refetch();
            void gpcQuery.refetch();
          }}
        />
      </AccountPage>
    );
  }

  const activeConsents = consentsQuery.data.consents.filter((item) => !item.revoked_at);

  return (
    <AccountPage
      title="Confidentialité"
      description="Contrôlez les traitements optionnels et exercez vos droits sur les données Account."
    >
      <section className="space-y-1">
        <h2 className="text-base font-semibold">Consentements optionnels</h2>
        <div className="divide-y divide-border border-y border-border">
          <TrackingConsentToggle
            large
            checked={consent.categories.analytics}
            label="Mesure d’audience"
            description="Aide à comprendre l’usage des produits sans stocker de données sensibles."
            onChange={() => updateConsent(consent, 'analytics')}
          />
          <TrackingConsentToggle
            large
            checked={consent.categories.performance}
            label="Diagnostic et performance"
            description="Autorise les rapports d’erreur et les mesures de fiabilité."
            onChange={() => updateConsent(consent, 'performance')}
          />
        </div>
      </section>

      <section className="space-y-3 border-b border-border pb-7">
        <h2 className="text-base font-semibold">Global Privacy Control</h2>
        <p className="text-sm text-muted-foreground">
          Signal détecté : {gpcQuery.data.gpc_enabled ? 'oui' : 'non'} · Opposition active :{' '}
          {gpcQuery.data.gpc_opt_out_active ? 'oui' : 'non'}
        </p>
      </section>

      <section className="space-y-3 border-b border-border pb-7">
        <h2 className="text-base font-semibold">Consentements actifs</h2>
        {activeConsents.length ? (
          <ul className="space-y-2 text-sm">
            {activeConsents.map((item) => (
              <li key={item.id} className="flex flex-wrap justify-between gap-2">
                <span className="font-medium text-foreground">{item.consent_type}</span>
                <span className="text-muted-foreground">
                  {formatDate(item.granted_at)} · {item.document_version}
                </span>
              </li>
            ))}
          </ul>
        ) : (
          <p className="text-sm text-muted-foreground">Aucun consentement optionnel actif.</p>
        )}
      </section>

      <PrivacyActions />
    </AccountPage>
  );
}

function PrivacyActions() {
  const [confirmation, setConfirmation] = useState('');
  const closureMutation = useMutation({
    mutationFn: () => accountClient.closeAccount(),
  });
  const closureQuery = useQuery({
    queryKey: accountQueryKeys.closure,
    queryFn: ({ signal }) => accountClient.getAccountClosure({ signal }),
    enabled: closureMutation.isSuccess,
    refetchInterval: (query) =>
      query.state.data && ['completed', 'failed', 'cancelled'].includes(query.state.data.status)
        ? false
        : 1_000,
  });
  useAccountAuthenticationRecovery(closureMutation.error ?? closureQuery.error);

  return (
    <section className="space-y-6">
      <AccountPrivacyExportActions />

      <div className="space-y-3 border-t border-destructive/30 pt-6">
        <div>
          <h2 className="text-base font-semibold text-destructive">Fermer le compte</h2>
          <p className="mt-1 text-sm text-muted-foreground">
            Cette opération engage la procédure définitive de fermeture. Saisissez FERMER pour
            confirmer.
          </p>
        </div>
        <div className="flex max-w-md gap-2">
          <Input
            value={confirmation}
            aria-label="Confirmation de fermeture"
            autoComplete="off"
            onChange={(event) => setConfirmation(event.currentTarget.value)}
          />
          <Button
            type="button"
            variant="destructive"
            disabled={
              confirmation !== 'FERMER' || closureMutation.isPending || closureMutation.isSuccess
            }
            onClick={() => closureMutation.mutate()}
          >
            {closureMutation.isPending ? 'Fermeture…' : 'Fermer'}
          </Button>
        </div>
        {closureMutation.error ? (
          <Alert variant="destructive">
            <AlertTitle>Fermeture impossible</AlertTitle>
            <AlertDescription>{closureMutation.error.message}</AlertDescription>
          </Alert>
        ) : null}
        {closureQuery.data?.status === 'failed' ? (
          <Alert variant="destructive">
            <AlertTitle>Fermeture interrompue</AlertTitle>
            <AlertDescription>
              {closureFailureMessage(closureQuery.data.last_error)}
            </AlertDescription>
          </Alert>
        ) : closureMutation.isSuccess ? (
          <Alert>
            <AlertTitle>Demande enregistrée</AlertTitle>
            <AlertDescription>
              {closureQuery.data
                ? `${closureQuery.data.participants.filter((step) => step.status === 'completed').length} étape(s) sur ${closureQuery.data.participants.length} terminée(s).`
                : 'La procédure de fermeture de compte a commencé.'}
            </AlertDescription>
          </Alert>
        ) : null}
      </div>
    </section>
  );
}

function closureFailureMessage(code: string | null): string {
  if (code === 'cloud_closure_conflict' || code === 'billing_closure_conflict') {
    return 'Transférez ou supprimez les espaces dont vous êtes propriétaire, puis relancez la fermeture.';
  }
  return 'La fermeture n’a pas pu être terminée. Réessayez ou contactez le support.';
}

function updateConsent(consent: CookieConsentState, category: 'analytics' | 'performance'): void {
  setTrackingConsent(toggleConsentCategory(consent, category), 'account-web:privacy');
}

function formatDate(value: string): string {
  return new Intl.DateTimeFormat('fr-FR', { dateStyle: 'medium' }).format(new Date(value));
}
