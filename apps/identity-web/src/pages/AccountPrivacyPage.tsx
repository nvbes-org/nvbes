import { type GpcStatus, identityClient, type UserConsent } from '@nvbes/identity-client';
import { useVirtualizer } from '@tanstack/react-virtual';
import { Download, Eye, FileText, ShieldAlert, XCircle } from 'lucide-react';
import { useEffect, useRef, useState } from 'react';
import { z } from 'zod';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Separator } from '@/components/ui/separator';
import { Skeleton } from '@/components/ui/skeleton';
import { identityHttpClient } from '../identity.http';
import { CookieConsentSettings } from '../components/CookieConsentSettings';
import { setTrackingConsent, DECLINE_ALL_CONSENT } from '../tracking-consent';

const emptySchema = z.undefined();

function exportAccountData(): Promise<void> {
  return identityHttpClient.post('/auth/me/export', emptySchema, {});
}

function deleteAccount(): Promise<void> {
  return identityHttpClient.post('/auth/me/delete', emptySchema, {});
}

const consentLabels: Record<string, string> = {
  terms_of_service: "Conditions d'utilisation",
  privacy_policy: 'Politique de confidentialite',
  cookie_consent: 'Consentement cookies global',
  cookie_consent_essentials: 'Cookies essentiels',
  cookie_consent_analytics: 'Cookies analytiques',
  cookie_consent_performance: 'Cookies de performance',
  cookie_consent_vendor_stripe: 'Cookie Stripe',
  cookie_consent_vendor_identity: 'Cookie nvbes Identity',
  cookie_consent_vendor_posthog: 'Cookie PostHog',
  cookie_consent_vendor_sentry: 'Cookie Sentry',
  posthog_product_analytics: 'PostHog analytics produit',
  posthog_autocapture_heatmaps: 'PostHog heatmaps & autocapture',
  posthog_session_replay: 'PostHog session replay',
  posthog_surveys_feedback: 'PostHog surveys & feedback',
  posthog_error_tracking: 'PostHog error tracking',
  posthog_feature_flags: 'PostHog feature flags',
  marketing_emails: 'Emails marketing',
  data_processing: 'Traitement des donnees',
  third_party_sharing: 'Partage avec des tiers',
  gpc_opt_out: 'Global Privacy Control — Ne pas vendre mes donnees',
};

function PrivacySkeleton() {
  return (
    <div className="flex flex-col gap-6">
      <div>
        <Skeleton className="h-6 w-56" />
        <Skeleton className="h-4 w-72 mt-1" />
      </div>
      <Card>
        <CardHeader>
          <Skeleton className="h-5 w-32" />
          <Skeleton className="h-4 w-64" />
        </CardHeader>
        <CardContent className="flex flex-col gap-3">
          {Array.from({ length: 3 }).map((_, i) => (
            <Skeleton key={i} className="h-12 w-full rounded-lg" />
          ))}
        </CardContent>
      </Card>
    </div>
  );
}

function ErrorMessage({ message }: { message: string }) {
  return (
    <div className="rounded-lg border border-destructive/30 bg-destructive/5 px-3 py-2 text-sm text-destructive">
      {message}
    </div>
  );
}

function SuccessMessage({ message }: { message: string }) {
  return (
    <div className="rounded-lg border border-emerald-500/30 bg-emerald-500/5 px-3 py-2 text-sm text-emerald-600">
      {message}
    </div>
  );
}

export default function AccountPrivacyPage() {
  const [consents, setConsents] = useState<UserConsent[]>([]);
  const [gpc, setGpc] = useState<GpcStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [exporting, setExporting] = useState(false);
  const [exportSuccess, setExportSuccess] = useState(false);
  const [exportError, setExportError] = useState<string | null>(null);
  const [showDeleteDialog, setShowDeleteDialog] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const [deleteConfirmText, setDeleteConfirmText] = useState('');
  const [deleteError, setDeleteError] = useState<string | null>(null);
  const [cookieConsentRevision, setCookieConsentRevision] = useState(0);
  const listRef = useRef<HTMLDivElement>(null);

  const fetchConsents = async () => {
    try {
      setConsents(await identityClient.listConsents());
    } finally {
      setLoading(false);
    }
  };

  const fetchGpcStatus = async () => {
    try {
      setGpc(await identityClient.gpcStatus());
    } catch {
      // GPC endpoint is best-effort; silently ignore failures
    }
  };

  useEffect(() => {
    void fetchConsents();
    void fetchGpcStatus();
  }, []);

  const rowCount = consents.length > 0 ? consents.length * 2 - 1 : 0;
  const virtualizer = useVirtualizer({
    count: rowCount,
    getScrollElement: () => listRef.current,
    estimateSize: (index) => (index % 2 === 1 ? 8 : 56),
    overscan: 8,
  });

  const handleRevoke = async (consent: UserConsent) => {
    const previous = consents;
    setConsents((prev) => prev.filter((c) => c.id !== consent.id));
    try {
      await identityClient.revokeConsent(consent.consent_type, consent.document_version);
      if (
        consent.consent_type === 'cookie_consent' ||
        consent.consent_type.startsWith('cookie_consent_')
      ) {
        // Also update local storage if cookie consent revoked
        setTrackingConsent(DECLINE_ALL_CONSENT, 'identity-web:account-privacy:revoke');
        setCookieConsentRevision((revision) => revision + 1);
      }
    } catch {
      setConsents(previous);
    }
  };

  const handleExport = async () => {
    setExporting(true);
    setExportError(null);
    setExportSuccess(false);
    try {
      await exportAccountData();
      setExportSuccess(true);
    } catch {
      setExportError("L'export de vos donnees a echoue. Veuillez reessayer.");
    } finally {
      setExporting(false);
    }
  };

  const handleDeleteAccount = async () => {
    if (deleteConfirmText !== 'SUPPRIMER') return;
    setDeleting(true);
    setDeleteError(null);
    try {
      await deleteAccount();
      window.location.href = '/login';
    } catch {
      setDeleteError('La suppression du compte a echoue. Veuillez reessayer.');
      setDeleting(false);
    }
  };

  const handleOpenDelete = () => {
    setDeleteConfirmText('');
    setDeleteError(null);
    setShowDeleteDialog(true);
  };

  if (loading) return <PrivacySkeleton />;

  return (
    <div className="flex flex-col gap-6 animate-fade-slide-up [animation-delay:0ms]">
      <div>
        <h1 className="text-xl font-heading font-semibold">Consentements & vie privee</h1>
        <p className="text-sm text-muted-foreground mt-1">
          Gerer vos consentements, vos donnees et vos droits RGPD.
        </p>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Consentements actifs</CardTitle>
          <CardDescription>
            Les consentements que vous avez accordes. Vous pouvez les revoquer a tout moment.
          </CardDescription>
        </CardHeader>
        <CardContent ref={listRef} className="max-h-[32rem] overflow-auto p-0">
          {consents.length === 0 ? (
            <div className="px-6 py-8">
              <div className="flex flex-col items-center gap-3">
                <XCircle className="size-8 text-muted-foreground" />
                <div className="text-center">
                  <p className="text-sm text-muted-foreground">Aucun consentement actif.</p>
                </div>
              </div>
            </div>
          ) : (
            <div
              style={{
                height: `${virtualizer.getTotalSize()}px`,
                position: 'relative',
              }}
            >
              {virtualizer.getVirtualItems().map((virtualItem) => {
                if (virtualItem.index % 2 === 1) {
                  return (
                    <div
                      key={`separator-${virtualItem.index}`}
                      className="absolute left-0 right-0 px-6"
                      style={{
                        transform: `translateY(${virtualItem.start}px)`,
                      }}
                    >
                      <Separator className="my-1" />
                    </div>
                  );
                }

                const consent = consents[Math.floor(virtualItem.index / 2)];
                const label = consentLabels[consent.consent_type] ?? consent.consent_type;
                const grantedDate = new Date(consent.granted_at).toLocaleDateString('fr-FR', {
                  year: 'numeric',
                  month: 'long',
                  day: 'numeric',
                });

                return (
                  <div
                    key={consent.id}
                    ref={virtualizer.measureElement}
                    data-index={virtualItem.index}
                    className="absolute left-0 right-0 px-6"
                    style={{ transform: `translateY(${virtualItem.start}px)` }}
                  >
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
                        onClick={() => handleRevoke(consent)}
                        aria-label={`Revoquer ${label}`}
                      >
                        <XCircle className="size-4" />
                      </Button>
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </CardContent>
      </Card>

      <Card className="animate-fade-slide-up [animation-delay:25ms]">
        <CardHeader>
          <CardTitle>Configuration des cookies & traceurs</CardTitle>
          <CardDescription>
            Gérez précisément vos choix par catégorie ou par fournisseur (vendors).
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <CookieConsentSettings key={cookieConsentRevision} />
        </CardContent>
      </Card>

      {gpc?.gpc_enabled && (
        <Card className="animate-fade-slide-up [animation-delay:50ms] border-emerald-500/20 bg-emerald-500/[0.02]">
          <CardHeader>
            <div className="flex items-center gap-2">
              <Eye className="size-4 text-emerald-500" />
              <CardTitle>Global Privacy Control detecte</CardTitle>
            </div>
            <CardDescription>
              Votre navigateur a signale via le signal GPC (Sec-GPC) que vous ne souhaitez pas que
              vos donnees personnelles soient vendues ou partagees. Ce choix a ete enregistre
              automatiquement.
            </CardDescription>
          </CardHeader>
          <CardContent>
            <p className="text-xs text-muted-foreground">
              Le Global Privacy Control (GPC) est un signal envoye par votre navigateur pour exercer
              votre droit de refus de vente de donnees (opt-out). Conformement au CCPA et aux
              legislations applicables, nvbes respecte ce signal et ne vend ni ne partage vos
              donnees a des tiers.
            </p>
          </CardContent>
        </Card>
      )}

      <Card className="animate-fade-slide-up [animation-delay:100ms]">
        <CardHeader>
          <CardTitle>Vos droits RGPD</CardTitle>
          <CardDescription>
            Conformement au reglement general sur la protection des donnees, vous disposez des
            droits suivants.
          </CardDescription>
        </CardHeader>
        <CardContent className="flex flex-col gap-1">
          <div className="flex items-center justify-between gap-3 py-1">
            <div className="flex items-center gap-3 min-w-0">
              <div className="flex size-8 shrink-0 items-center justify-center rounded-lg bg-muted">
                <Download className="size-4 text-muted-foreground" />
              </div>
              <div className="flex flex-col min-w-0">
                <span className="text-sm font-medium">Exporter mes donnees</span>
                <span className="text-xs text-muted-foreground">
                  Recevez une copie de vos donnees personnelles au format JSON.
                </span>
              </div>
            </div>
            <Button
              variant="outline"
              size="sm"
              className="shrink-0"
              onClick={handleExport}
              disabled={exporting}
            >
              {exporting ? 'Export...' : 'Exporter'}
            </Button>
          </div>
          {exportSuccess && (
            <SuccessMessage message="Votre demande d'export a ete enregistree. Vous recevrez vos donnees par email dans un delai de 30 jours." />
          )}
          {exportError && <ErrorMessage message={exportError} />}

          <Separator className="my-1" />

          <div className="flex items-center justify-between gap-3 py-1">
            <div className="flex items-center gap-3 min-w-0">
              <div className="flex size-8 shrink-0 items-center justify-center rounded-lg bg-muted">
                <ShieldAlert className="size-4 text-destructive/70" />
              </div>
              <div className="flex flex-col min-w-0">
                <span className="text-sm font-medium">Supprimer mon compte</span>
                <span className="text-xs text-muted-foreground">
                  Supprimez definitivement votre compte et toutes vos donnees.
                </span>
              </div>
            </div>
            <Button
              variant="outline"
              size="sm"
              className="shrink-0 border-destructive/30 text-destructive hover:bg-destructive/10"
              onClick={handleOpenDelete}
            >
              Supprimer
            </Button>
          </div>
        </CardContent>
      </Card>

      <Dialog open={showDeleteDialog} onOpenChange={setShowDeleteDialog}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Supprimer votre compte</DialogTitle>
            <DialogDescription>
              Cette action est irreversible. Toutes vos donnees, workspaces et abonnements seront
              definitivement supprimes. Tapez SUPPRIMER pour confirmer.
            </DialogDescription>
          </DialogHeader>
          <div className="flex flex-col gap-4 py-4">
            <input
              type="text"
              className="flex h-10 w-full rounded-lg border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
              placeholder="SUPPRIMER"
              value={deleteConfirmText}
              onChange={(e) => setDeleteConfirmText(e.target.value)}
              autoFocus
            />
            {deleteError && <ErrorMessage message={deleteError} />}
          </div>
          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              onClick={() => setShowDeleteDialog(false)}
              disabled={deleting}
            >
              Annuler
            </Button>
            <Button
              type="button"
              variant="default"
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
              onClick={handleDeleteAccount}
              disabled={deleting || deleteConfirmText !== 'SUPPRIMER'}
            >
              {deleting ? 'Suppression...' : 'Supprimer definitivement'}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
