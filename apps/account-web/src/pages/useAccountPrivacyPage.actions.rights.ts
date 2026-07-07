import { deleteAccount, exportAccountData } from './AccountPrivacyPage.shared';

export function buildAccountPrivacyRightsActions({
  deleteConfirmText,
  setExporting,
  setExportError,
  setExportSuccess,
  setDeleting,
  setDeleteError,
  setDeleteConfirmText,
  setShowDeleteDialog,
}: {
  deleteConfirmText: string;
  setExporting: React.Dispatch<React.SetStateAction<boolean>>;
  setExportError: React.Dispatch<React.SetStateAction<string | null>>;
  setExportSuccess: React.Dispatch<React.SetStateAction<boolean>>;
  setDeleting: React.Dispatch<React.SetStateAction<boolean>>;
  setDeleteError: React.Dispatch<React.SetStateAction<string | null>>;
  setDeleteConfirmText: React.Dispatch<React.SetStateAction<string>>;
  setShowDeleteDialog: React.Dispatch<React.SetStateAction<boolean>>;
}) {
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
    if (deleteConfirmText !== 'SUPPRIMER') {
      return;
    }

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

  return {
    handleExport,
    handleDeleteAccount,
    handleOpenDelete,
  };
}
