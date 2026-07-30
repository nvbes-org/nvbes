import { Card, CardContent } from '@/components/ui/card';
import { Skeleton } from '@/components/ui/skeleton';

export function PersonalInfoSkeleton() {
  return (
    <div className="flex flex-col gap-6">
      <div>
        <Skeleton className="mt-1 h-4 w-72" />
        <Skeleton className="h-6 w-48" />
      </div>
      <Card>
        <CardContent className="flex flex-col gap-3 pt-6">
          {Array.from({ length: 5 }).map((_, index) => (
            <Skeleton key={index} className="h-10 w-full rounded-lg" />
          ))}
        </CardContent>
      </Card>
    </div>
  );
}
