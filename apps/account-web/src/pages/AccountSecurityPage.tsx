import { SecurityMfaCard, SecuritySignInOptionsCard } from './AccountSecurityPage.shared';
import { useAccountSecurityPage } from './useAccountSecurityPage';

export default function AccountSecurityPage() {
  const { isPending, mutation, onOpenMfa, onOpenPassword, onOpenSessions, overview } =
    useAccountSecurityPage();

  return (
    <div className="flex flex-col gap-6 animate-fade-slide-up [animation-delay:0ms]">
      <div>
        <h1 className="text-3xl font-heading font-semibold">Sécurité</h1>
      </div>
      <SecurityMfaCard
        overview={overview}
        disabled={isPending}
        onOpenMfa={onOpenMfa}
        onOpenPassword={onOpenPassword}
        onOpenSessions={onOpenSessions}
      />

      <div>
        <h1 className="text-2xl font-heading font-semibold">Options de connexion</h1>
        <p className="text-sm text-muted-foreground mt-1">
          Personnalisez la facon dont vous vous authentifiez.
        </p>
      </div>

      <SecuritySignInOptionsCard
        overview={overview}
        pending={mutation.isPending}
        onSkipPasswordChange={(checked) => mutation.mutate(checked)}
      />
    </div>
  );
}
