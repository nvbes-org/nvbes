import { Loader2 } from 'lucide-react';
import type { ComponentProps } from 'react';
import { cn } from './lib/classnames';

type ButtonVariant = 'default' | 'ghost' | 'outline';
type ButtonSize = 'default' | 'sm' | 'icon-sm';

const buttonVariantClass: Record<ButtonVariant, string> = {
  default: 'bg-primary text-primary-foreground hover:opacity-90',
  ghost: 'text-muted-foreground hover:bg-muted hover:text-foreground',
  outline: 'border border-border bg-background text-foreground hover:bg-muted',
};

const buttonSizeClass: Record<ButtonSize, string> = {
  default: 'h-8 px-2.5 text-sm',
  sm: 'h-7 px-2.5 text-[0.8rem]',
  'icon-sm': 'size-7 p-0',
};

export function SwitcherButton({
  className,
  variant = 'default',
  size = 'default',
  type = 'button',
  ...props
}: ComponentProps<'button'> & {
  variant?: ButtonVariant;
  size?: ButtonSize;
}) {
  return (
    <button
      type={type}
      className={cn(
        'p-6 inline-flex shrink-0 items-center justify-center gap-1.5 rounded-lg font-medium transition outline-none disabled:pointer-events-none disabled:opacity-50',
        buttonVariantClass[variant],
        buttonSizeClass[size],
        className,
      )}
      {...props}
    />
  );
}

export function SwitcherCard({ className, ...props }: ComponentProps<'div'>) {
  return (
    <div
      className={cn('rounded-xl bg-card text-card-foreground ring-1 ring-foreground/10', className)}
      {...props}
    />
  );
}

export function SwitcherBadge({ className, ...props }: ComponentProps<'span'>) {
  return (
    <span
      className={cn(
        'inline-flex h-5 w-fit shrink-0 items-center justify-center rounded-full border border-border bg-background px-2 text-xs font-medium text-foreground',
        className,
      )}
      {...props}
    />
  );
}

export function SwitcherAvatar({ className, ...props }: ComponentProps<'div'>) {
  return (
    <div
      className={cn(
        'flex shrink-0 items-center justify-center rounded-full border border-border bg-muted text-muted-foreground',
        className,
      )}
      {...props}
    />
  );
}

export function SwitcherSeparator({ className, ...props }: ComponentProps<'div'>) {
  return <div className={cn('h-px w-full bg-border', className)} {...props} />;
}

export function SwitcherSpinner({ className, ...props }: ComponentProps<typeof Loader2>) {
  return (
    <Loader2
      role="status"
      aria-label="Loading"
      className={cn('size-4 animate-spin', className)}
      {...props}
    />
  );
}
