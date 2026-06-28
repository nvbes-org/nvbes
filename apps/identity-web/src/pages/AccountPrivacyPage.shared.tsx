import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { CookieConsentSettings } from '../components/CookieConsentSettings';
import { AccountPrivacyConsentList } from './AccountPrivacyConsentList';
import { GpcStatusCard, PrivacyRightsCard } from './AccountPrivacyPage.cards';
import { DeleteAccountDialog } from './AccountPrivacyPage.dialog';
import type { useAccountPrivacyPage } from './useAccountPrivacyPage';
export { deleteAccount, exportAccountData } from './AccountPrivacyPage.api';
export {
  ErrorMessage,
  GpcStatusCard,
  PrivacyRightsCard,
  PrivacySkeleton,
  SuccessMessage,
} from './AccountPrivacyPage.cards';
export {
  consentLabels,
  ConsentEmptyState,
  ConsentRow,
  isVisibleConsentType,
} from './AccountPrivacyPage.consents';
export { DeleteAccountDialog } from './AccountPrivacyPage.dialog';

type AccountPrivacyPageContentProps = ReturnType<typeof useAccountPrivacyPage> & {
  onExport: () => void;
  onDelete: () => void;
};

export function AccountPrivacyPageContent({
  consents,
  gpc,
  cookieConsentRevision,
  exporting,
  exportSuccess,
  exportError,
  showDeleteDialog,
  deleting,
  deleteConfirmText,
  deleteError,
  handleRevoke,
  onExport,
  handleOpenDelete,
  onDelete,
  setShowDeleteDialog,
  setDeleteConfirmText,
}: AccountPrivacyPageContentProps) {
  return (
    <div className="flex animate-fade-slide-up flex-col gap-6 [animation-delay:0ms]">
      <div>
        <h1 className="font-heading text-xl font-semibold">Consentements & vie privee</h1>
        <p className="mt-1 text-sm text-muted-foreground">
          Gerer vos consentements, vos donnees et vos droits RGPD.
        </p>
      </div>

      <AccountPrivacyConsentList consents={consents} onRevoke={handleRevoke} />

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

      <GpcStatusCard gpc={gpc} />

      <PrivacyRightsCard
        exporting={exporting}
        exportSuccess={exportSuccess}
        exportError={exportError}
        onExport={onExport}
        onOpenDelete={handleOpenDelete}
      />

      <DeleteAccountDialog
        open={showDeleteDialog}
        deleting={deleting}
        deleteConfirmText={deleteConfirmText}
        deleteError={deleteError}
        onOpenChange={setShowDeleteDialog}
        onDeleteConfirmTextChange={setDeleteConfirmText}
        onDelete={onDelete}
      />
    </div>
  );
}
