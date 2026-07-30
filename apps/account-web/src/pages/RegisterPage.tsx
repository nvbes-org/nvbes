import { useLocation } from '@tanstack/react-router';
import { RegisterBrandPanel } from './RegisterBrandPanel';
import { RegisterForm } from './RegisterPage.form';
import { RegisterPageShell } from './RegisterPage.layout';
import { useRegisterPage } from './useRegisterPage';

export default function RegisterPage() {
  const location = useLocation();
  const page = useRegisterPage();

  if (page.checkingAuth) {
    return null;
  }

  return (
    <div className="flex min-h-screen">
      <RegisterBrandPanel />

      <div className="flex flex-1 flex-col items-center justify-center w-full">
        <RegisterPageShell title="Inscription" searchStr={location.searchStr}>
          <RegisterForm {...page} />
        </RegisterPageShell>
      </div>
    </div>
  );
}
