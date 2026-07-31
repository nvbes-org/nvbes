import { IdentityPage, IdentityPageHeader } from '@/components/IdentityPage';

import { SecurityMfaCard, SecuritySignInOptionsCard } from './IdentitySecurityPage.shared';
import { useIdentitySecurityPage } from './useIdentitySecurityPage';

export default function IdentitySecurityPage() {
  const { isPending, mutation, onOpenMfa, onOpenPassword, onOpenSessions, overview } =
    useIdentitySecurityPage();

  return (
    <IdentityPage>
      <IdentityPageHeader title="Sécurité" />
      <SecurityMfaCard
        overview={overview}
        disabled={isPending}
        onOpenMfa={onOpenMfa}
        onOpenPassword={onOpenPassword}
        onOpenSessions={onOpenSessions}
      />

      <IdentityPageHeader
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
    </IdentityPage>
  );
}
