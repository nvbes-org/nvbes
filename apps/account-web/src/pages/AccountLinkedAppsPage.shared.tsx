import { AccountCardListSkeleton } from '@/components/AccountCardListSkeleton';

export { LinkedAppsList } from './AccountLinkedAppsPage.list';

export function LinkedAppsSkeleton() {
  return (
    <AccountCardListSkeleton titleClassName="mt-1 h-6 w-32" descriptionClassName="mt-1 h-4 w-64" />
  );
}
