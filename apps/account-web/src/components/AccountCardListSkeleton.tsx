import { Card, CardContent, CardHeader } from '@/components/ui/card';
import { Skeleton } from '@/components/ui/skeleton';
import type { ReactNode } from 'react';
import { SkeletonGroup } from './SkeletonGroup';

export function AccountCardSkeleton({
  titleClassName,
  descriptionClassName,
  contentClassName,
  children,
}: {
  titleClassName: string;
  descriptionClassName: string;
  contentClassName: string;
  children: ReactNode;
}) {
  return (
    <Card>
      <CardHeader>
        <Skeleton className={titleClassName} />
        <Skeleton className={descriptionClassName} />
      </CardHeader>
      <CardContent className={contentClassName}>{children}</CardContent>
    </Card>
  );
}

export function AccountCardListSkeleton({
  titleClassName,
  descriptionClassName,
  itemCount = 3,
}: {
  titleClassName: string;
  descriptionClassName: string;
  itemCount?: number;
}) {
  return (
    <div className="flex flex-col gap-6">
      <div>
        <Skeleton className={titleClassName} />
        <Skeleton className={descriptionClassName} />
      </div>
      <AccountCardSkeleton
        titleClassName="h-5 w-32"
        descriptionClassName="h-4 w-48"
        contentClassName="flex flex-col gap-3"
      >
        <SkeletonGroup count={itemCount} itemClassName="h-12 w-full rounded-lg" />
      </AccountCardSkeleton>
    </div>
  );
}
