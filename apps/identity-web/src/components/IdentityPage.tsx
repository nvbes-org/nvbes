import { ArrowLeft } from 'lucide-react';
import type { ComponentProps, ElementType, ReactNode } from 'react';
import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';

export function IdentityPage({ className, ...props }: ComponentProps<'div'>) {
  return (
    <div
      className={cn('flex animate-fade-slide-up flex-col gap-6 [animation-delay:0ms]', className)}
      {...props}
    />
  );
}

export function IdentityPageHeader({
  title,
  description,
  action,
  onBack,
  backLabel = 'Retour',
  size = 'page',
  as: Heading = 'h1',
  className,
  contentClassName,
}: {
  title: ReactNode;
  description?: ReactNode;
  action?: ReactNode;
  onBack?: () => void;
  backLabel?: string;
  size?: 'page' | 'section' | 'subsection';
  as?: ElementType;
  className?: string;
  contentClassName?: string;
}) {
  return (
    <header
      className={cn(action && 'flex flex-wrap items-center justify-between gap-4', className)}
    >
      <div className={contentClassName}>
        {onBack ? (
          <Button
            type="button"
            variant="ghost"
            size="sm"
            className="mb-3 -ml-1 text-muted-foreground"
            onClick={onBack}
          >
            <ArrowLeft data-icon="inline-start" />
            {backLabel}
          </Button>
        ) : null}
        <Heading
          className={cn(
            'font-heading font-semibold',
            size === 'page' && 'text-3xl',
            size === 'section' && 'text-xl',
            size === 'subsection' && 'text-2xl',
          )}
        >
          {title}
        </Heading>
        {description ? <p className="mt-1 text-sm text-muted-foreground">{description}</p> : null}
      </div>
      {action}
    </header>
  );
}
