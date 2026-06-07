import { Button } from '@/components/ui/button';
import { ResetPasswordSuccessMessage } from './ResetPasswordPage.messages';

export function ResetPasswordSuccessState({ navigateToLogin }: { navigateToLogin: () => void }) {
  return (
    <div className="flex flex-col gap-5">
      <ResetPasswordSuccessMessage message="Votre mot de passe a ete reinitialise avec succes." />
      <Button className="w-full" onClick={navigateToLogin}>
        Se connecter
      </Button>
    </div>
  );
}
