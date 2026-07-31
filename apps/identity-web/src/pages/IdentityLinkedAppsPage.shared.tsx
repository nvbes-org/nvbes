import { IdentityCardListSkeleton } from '@/components/IdentityCardListSkeleton';

export { LinkedAppsList } from './IdentityLinkedAppsPage.list';

export function LinkedAppsSkeleton() {
  return (
    <IdentityCardListSkeleton titleClassName="mt-1 h-6 w-32" descriptionClassName="mt-1 h-4 w-64" />
  );
}
