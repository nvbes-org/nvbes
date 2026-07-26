import type { GpcStatus, UserConsent } from '@nvbes/identity-client';
import { useState } from 'react';
import { useAccountPrivacyPageActions } from './useAccountPrivacyPage.actions';
import { useAccountPrivacyPageLoad } from './useAccountPrivacyPage.load';

export function useAccountPrivacyPage() {
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
  const handleCookieConsentChange = () => {
    setCookieConsentRevision((revision) => revision + 1);
  };

  useAccountPrivacyPageLoad({
    setConsents,
    setGpc,
    setLoading,
  });

  const actions = useAccountPrivacyPageActions({
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
  });

  return {
    consents,
    gpc,
    loading,
    exporting,
    exportSuccess,
    exportError,
    showDeleteDialog,
    deleting,
    deleteConfirmText,
    deleteError,
    cookieConsentRevision,
    handleCookieConsentChange,
    setShowDeleteDialog,
    setDeleteConfirmText,
    ...actions,
  };
}
