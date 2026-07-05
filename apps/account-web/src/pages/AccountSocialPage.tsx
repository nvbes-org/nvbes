import { SocialLinkedIdentitiesCard, SocialSkeleton } from './AccountSocialPage.shared';
import { useAccountSocialPage } from './useAccountSocialPage';

export default function AccountSocialPage() {
  const { accountLoading, identities, isPending, me, tenantId, unlinking, handleUnlink } =
    useAccountSocialPage();

  if (accountLoading || (tenantId && isPending)) return <SocialSkeleton />;

  return (
    <div className="flex flex-col gap-6 animate-fade-slide-up [animation-delay:0ms]">
      <div>
        <h1 className="text-xl font-heading font-semibold">Contenu & social</h1>
        <p className="text-sm text-muted-foreground mt-1">
          Gerer vos profils sociaux et identites liees.
        </p>
      </div>

      <SocialLinkedIdentitiesCard
        identities={identities}
        hasTenant={Boolean(me?.current_tenant_id)}
        unlinking={unlinking}
        onUnlink={handleUnlink}
      />
    </div>
  );
}
