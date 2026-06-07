import { useAccountContext } from '@/hooks/useAccountContext';
import { AccountStandingContent, StandingSkeleton } from '@/pages/AccountStandingPage.shared';

export default function AccountStandingPage() {
  const { me } = useAccountContext();

  if (!me) {
    return <StandingSkeleton />;
  }

  return <AccountStandingContent me={me} />;
}
