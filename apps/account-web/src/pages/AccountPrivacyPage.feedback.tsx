import { AccountCardSkeleton } from '@/components/AccountCardListSkeleton';
import { SkeletonGroup } from '@/components/SkeletonGroup';
import { Skeleton } from '@/components/ui/skeleton';

export function PrivacySkeleton() {
  return (
    <div className="flex flex-col gap-8">
      <div className="flex flex-col gap-3">
        <Skeleton className="h-5 w-40" />
        <Skeleton className="h-7 w-64" />
        <Skeleton className="h-4 w-full max-w-xl" />
      </div>
      <AccountCardSkeleton
        titleClassName="h-5 w-48"
        descriptionClassName="h-4 w-full max-w-md"
        contentClassName="grid gap-3 md:grid-cols-3"
      >
        <SkeletonGroup count={3} itemClassName="h-16 w-full rounded-lg" />
      </AccountCardSkeleton>
      <div className="flex flex-col gap-6">
        <SkeletonGroup
          itemClassNames={[
            'h-[34rem] w-full rounded-xl',
            'h-12 w-full',
            'h-48 w-full rounded-xl',
            'h-48 w-full rounded-xl',
          ]}
        />
      </div>
    </div>
  );
}
