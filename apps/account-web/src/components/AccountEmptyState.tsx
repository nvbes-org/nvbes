import type { ComponentType, ReactNode } from 'react';
import {
  Empty,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from '@/components/ui/empty';

export function AccountEmptyState({
  icon: Icon,
  title,
  description,
  className,
}: {
  icon: ComponentType<{ className?: string }>;
  title: ReactNode;
  description: ReactNode;
  className?: string;
}) {
  return (
    <Empty className={className}>
      <EmptyHeader>
        <EmptyMedia variant="icon">
          <Icon />
        </EmptyMedia>
        <EmptyTitle>{title}</EmptyTitle>
        <EmptyDescription>{description}</EmptyDescription>
      </EmptyHeader>
    </Empty>
  );
}
