import { LinkedAppsList, LinkedAppsSkeleton } from '@/pages/IdentityLinkedAppsPage.shared';
import { useIdentityLinkedAppsPage } from '@/pages/useIdentityLinkedAppsPage';

export default function IdentityLinkedAppsPage() {
  const { clients, handleRevoke, isPending, listRef, revoking, virtualizer } =
    useIdentityLinkedAppsPage();

  if (isPending) {
    return <LinkedAppsSkeleton />;
  }

  return (
    <LinkedAppsList
      clients={clients}
      listRef={listRef}
      revoking={revoking}
      virtualizer={virtualizer}
      onRevoke={handleRevoke}
    />
  );
}
