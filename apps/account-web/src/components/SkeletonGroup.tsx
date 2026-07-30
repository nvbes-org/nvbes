import { Skeleton } from '@/components/ui/skeleton';

export function SkeletonGroup({
  count,
  itemClassName,
  itemClassNames,
}: {
  count?: number;
  itemClassName?: string;
  itemClassNames?: string[];
}) {
  const classNames =
    itemClassNames ?? Array.from({ length: count ?? 0 }, () => itemClassName ?? '');
  return classNames.map((className, index) => <Skeleton key={index} className={className} />);
}
