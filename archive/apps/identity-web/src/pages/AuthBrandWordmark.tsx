import type { ElementType } from 'react';
import { cn } from '@/lib/utils';

export function AuthBrandWordmark({
  as: Component = 'h2',
  className,
}: {
  as?: ElementType;
  className?: string;
}) {
  return (
    <Component
      className={cn(
        'auth-brand-wordmark text-4xl leading-tight font-bold tracking-tight',
        className,
      )}
      data-wordmark="nvbes"
    >
      nvbes
    </Component>
  );
}
