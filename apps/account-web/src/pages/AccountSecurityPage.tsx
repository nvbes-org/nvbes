import { SecurityMfaCard, SecuritySignInOptionsCard } from './AccountSecurityPage.shared';
import { useAccountSecurityPage } from './useAccountSecurityPage';

export default function AccountSecurityPage() {
  const { isPending, mutation, onOpenMfa, onOpenPassword, onOpenSessions, overview } =
    useAccountSecurityPage();

  return (
    <div className="flex flex-col gap-6 animate-fade-slide-up [animation-delay:0ms]">
      <div>
        <h1 className="text-xl font-heading font-semibold">Securite</h1>
        <p className="text-sm text-muted-foreground mt-1">
          Gerer l&apos;authentification et la securite de votre compte.
        </p>
      </div>
      <SecurityMfaCard
        overview={overview}
        disabled={isPending}
        onOpenMfa={onOpenMfa}
        onOpenPassword={onOpenPassword}
        onOpenSessions={onOpenSessions}
      />
      <SecuritySignInOptionsCard
        overview={overview}
        pending={mutation.isPending}
        onSkipPasswordChange={(checked) => mutation.mutate(checked)}
      />
    </div>
  );
}
