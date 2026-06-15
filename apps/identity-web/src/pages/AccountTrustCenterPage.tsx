import {
  TrustCenterHero,
  TrustCenterLegal,
  TrustCenterOperations,
  TrustCenterSecurityGrid,
} from './AccountTrustCenterPage.shared';
import { useAccountTrustCenterPage } from './useAccountTrustCenterPage';

export default function AccountTrustCenterPage() {
  const { data: trustCenter } = useAccountTrustCenterPage();

  return (
    <div className="flex flex-col gap-6 animate-fade-slide-up [animation-delay:0ms]">
      <div>
        <h1 className="text-xl font-heading font-semibold">Trust Center</h1>
        <p className="text-sm text-muted-foreground mt-1">
          Statut securite, conformite et hebergement genere depuis le tenant actif.
        </p>
      </div>
      <TrustCenterHero trustCenter={trustCenter} />
      <TrustCenterSecurityGrid trustCenter={trustCenter} />
      <TrustCenterOperations trustCenter={trustCenter} />
      <TrustCenterLegal trustCenter={trustCenter} />
    </div>
  );
}
