import { FeedbackAlert } from '@/components/FeedbackAlert';
import { Button } from '@/components/ui/button';

export function ResetPasswordSuccessState({ navigateToLogin }: { navigateToLogin: () => void }) {
  return (
    <div className="flex flex-col gap-5">
      <FeedbackAlert>Votre mot de passe a ete reinitialise avec succes.</FeedbackAlert>
      <Button className="w-full" onClick={navigateToLogin}>
        Se connecter
      </Button>
    </div>
  );
}
