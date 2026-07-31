import { IdentityPasswordPageContent } from '@/pages/IdentityPasswordPage.shared';
import { useIdentityPasswordPage } from '@/pages/useIdentityPasswordPage';

export default function IdentityPasswordPage() {
  return <IdentityPasswordPageContent {...useIdentityPasswordPage()} />;
}
