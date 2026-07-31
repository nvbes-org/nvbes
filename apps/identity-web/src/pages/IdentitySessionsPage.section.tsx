import type { ReactNode } from 'react';
import { cn } from '@/lib/utils';

export function DeviceSection({
  title,
  description,
  children,
  className,
}: {
  title: string;
  description: string;
  children: ReactNode;
  className?: string;
}) {
  return (
    <section className={cn('flex flex-col gap-3', className)}>
      <header>
        <h2 className="text-lg font-semibold tracking-tight">{title}</h2>
        <p className="text-xs text-muted-foreground">{description}</p>
      </header>
      {children}
    </section>
  );
}
