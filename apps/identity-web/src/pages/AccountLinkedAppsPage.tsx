import { LinkedAppsList, LinkedAppsSkeleton } from '@/pages/AccountLinkedAppsPage.shared';
import { useAccountLinkedAppsPage } from '@/pages/useAccountLinkedAppsPage';

export default function AccountLinkedAppsPage() {
  const { clients, handleRevoke, isPending, listRef, revoking, virtualizer } =
    useAccountLinkedAppsPage();

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
