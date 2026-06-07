import type { UserConsent } from '@nvbes/identity-client';
import { buildAccountPrivacyConsentActions } from './useAccountPrivacyPage.actions.consent';
import { buildAccountPrivacyRightsActions } from './useAccountPrivacyPage.actions.rights';

export function useAccountPrivacyPageActions({
  consents,
  setConsents,
  setExporting,
  setExportError,
  setExportSuccess,
  deleteConfirmText,
  setDeleting,
  setDeleteError,
  setDeleteConfirmText,
  setShowDeleteDialog,
  setCookieConsentRevision,
}: {
  consents: UserConsent[];
  setConsents: React.Dispatch<React.SetStateAction<UserConsent[]>>;
  setExporting: React.Dispatch<React.SetStateAction<boolean>>;
  setExportError: React.Dispatch<React.SetStateAction<string | null>>;
  setExportSuccess: React.Dispatch<React.SetStateAction<boolean>>;
  deleteConfirmText: string;
  setDeleting: React.Dispatch<React.SetStateAction<boolean>>;
  setDeleteError: React.Dispatch<React.SetStateAction<string | null>>;
  setDeleteConfirmText: React.Dispatch<React.SetStateAction<string>>;
  setShowDeleteDialog: React.Dispatch<React.SetStateAction<boolean>>;
  setCookieConsentRevision: React.Dispatch<React.SetStateAction<number>>;
}) {
  const consentActions = buildAccountPrivacyConsentActions({
    consents,
    setConsents,
    setCookieConsentRevision,
  });

  const rightsActions = buildAccountPrivacyRightsActions({
    deleteConfirmText,
    setExporting,
    setExportError,
    setExportSuccess,
    setDeleting,
    setDeleteError,
    setDeleteConfirmText,
    setShowDeleteDialog,
  });

  return {
    ...consentActions,
    ...rightsActions,
  };
}
