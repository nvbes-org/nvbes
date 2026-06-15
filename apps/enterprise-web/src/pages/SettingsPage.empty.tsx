import { AlertCircle } from 'lucide-react';
import {
  Empty,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from '../components/ui/empty';

export function EmptyState({ title, description }: { title: string; description: string }) {
  return (
    <Empty className="min-h-48 border bg-muted/20">
      <EmptyMedia variant="icon">
        <AlertCircle className="size-4" />
      </EmptyMedia>
      <EmptyHeader>
        <EmptyTitle>{title}</EmptyTitle>
        <EmptyDescription>{description}</EmptyDescription>
      </EmptyHeader>
    </Empty>
  );
}
