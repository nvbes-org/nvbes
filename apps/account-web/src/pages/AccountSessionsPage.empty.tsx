import { Laptop } from 'lucide-react';
import {
  Empty,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from '@/components/ui/empty';

export function EmptySessionsCard() {
  return (
    <Empty className="border">
      <EmptyHeader>
        <EmptyMedia variant="icon">
          <Laptop />
        </EmptyMedia>
        <EmptyTitle>Aucune session</EmptyTitle>
        <EmptyDescription>Impossible de charger vos sessions.</EmptyDescription>
      </EmptyHeader>
    </Empty>
  );
}
