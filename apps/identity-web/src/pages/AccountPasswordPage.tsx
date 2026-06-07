import { AccountPasswordPageContent } from '@/pages/AccountPasswordPage.shared';
import { useAccountPasswordPage } from '@/pages/useAccountPasswordPage';

export default function AccountPasswordPage() {
  return <AccountPasswordPageContent {...useAccountPasswordPage()} />;
}
