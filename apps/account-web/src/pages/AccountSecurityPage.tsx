import { SecurityMfaCard, SecuritySignInOptionsCard } from './AccountSecurityPage.shared';
import { useAccountSecurityPage } from './useAccountSecurityPage';

export default function AccountSecurityPage() {
  const { isPending, mutation, onOpenMfa, onOpenPassword, onOpenSessions, overview } =
    useAccountSecurityPage();

  return (
    <AccountPage>
      <AccountPageHeader title="Sécurité" />
      <SecurityMfaCard
        overview={overview}
        disabled={isPending}
        onOpenMfa={onOpenMfa}
        onOpenPassword={onOpenPassword}
        onOpenSessions={onOpenSessions}
      />

      <AccountPageHeader
        as="h2"
        size="subsection"
        title="Options de connexion"
        description="Personnalisez la facon dont vous vous authentifiez."
      />

      <SecuritySignInOptionsCard
        overview={overview}
        pending={mutation.isPending}
        onSkipPasswordChange={(checked) => mutation.mutate(checked)}
      />
    </AccountPage>
  );
}
import { AccountPage, AccountPageHeader } from '@/components/AccountPage';
