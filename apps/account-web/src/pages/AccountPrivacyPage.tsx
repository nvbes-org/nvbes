import { AccountPrivacyPageContent, PrivacySkeleton } from './AccountPrivacyPage.shared';
import { useAccountPrivacyPage } from './useAccountPrivacyPage';

export default function AccountPrivacyPage() {
  const page = useAccountPrivacyPage();

  if (page.loading) {
    return <PrivacySkeleton />;
  }

  return (
    <AccountPrivacyPageContent
      {...page}
      onExport={() => void page.handleExport()}
      onDelete={() => void page.handleDeleteAccount()}
    />
  );
}
