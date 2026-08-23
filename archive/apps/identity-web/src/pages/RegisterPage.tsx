import { useLocation } from '@tanstack/react-router';
import { AuthPageShell } from '@/components/AuthPageShell';
import { LoginPageLegalLinks } from './LoginPage.forms';
import { RegisterBrandPanel } from './RegisterBrandPanel';
import { RegisterForm } from './RegisterPage.form';
import { useRegisterPage } from './useRegisterPage';

export default function RegisterPage() {
  const location = useLocation();
  const page = useRegisterPage();

  if (page.checkingAuth) {
    return null;
  }

  return (
    <AuthPageShell brand={<RegisterBrandPanel />} footerLinks={<LoginPageLegalLinks />}>
      <RegisterForm {...page} loginTo={`/login${location.searchStr}`} />
    </AuthPageShell>
  );
}
