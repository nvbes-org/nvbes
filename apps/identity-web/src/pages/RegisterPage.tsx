import { useLocation } from '@tanstack/react-router';
import { RegisterBrandPanel } from './RegisterBrandPanel';
import { RegisterPageContent } from './RegisterPage.content';
import { RegisterPageShell } from './RegisterPage.layout';
import { RegisterProgress } from './RegisterPage.shared';
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
      <RegisterPageShell
        title={page.step === 1 ? 'Créer un compte' : 'Espace de travail'}
        description={
          page.step === 1
            ? 'Renseignez vos informations personnelles.'
            : 'Configurez votre espace de travail.'
        }
        searchStr={location.searchStr}
      >
        <RegisterProgress step={page.step} />
        <div className="mt-6">
          <RegisterPageContent {...page} />
        </div>
      </RegisterPageShell>
    </div>
  );
}
