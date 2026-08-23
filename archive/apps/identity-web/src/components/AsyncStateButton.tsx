import { CheckIcon } from 'lucide-react';
import type { ComponentProps, ReactNode } from 'react';
import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';

export type AsyncButtonState = 'idle' | 'pending' | 'success' | 'disabled';

type AsyncStateButtonProps = Omit<
  ComponentProps<typeof Button>,
  'children' | 'size' | 'variant'
> & {
  message: string;
  icon?: ReactNode;
  state: AsyncButtonState;
  variant?: ComponentProps<typeof Button>['variant'];
  size?: ComponentProps<typeof Button>['size'];
};

export function AsyncStateButton({
  className,
  disabled,
  icon,
  message,
  size,
  state,
  variant,
  ...props
}: AsyncStateButtonProps) {
  return (
    <Button
      {...props}
      variant={variant}
      size={size}
      disabled={disabled || state === 'pending' || state === 'disabled'}
      data-state={state}
      className={cn('relative', className)}
    >
      <span
        className={cn(
          'inset-0 flex items-center justify-center transition-opacity duration-200 ease-out',
          state === 'idle' || state === 'disabled'
            ? 'opacity-100'
            : 'pointer-events-none opacity-0',
        )}
      >
        <span aria-live={state === 'idle' ? 'polite' : undefined}>
          {icon ? (
            <>
              <span className="sr-only">{message}</span>
              <span aria-hidden="true">{icon}</span>
            </>
          ) : (
            message
          )}
        </span>
      </span>
      <span
        className={cn(
          'absolute inset-0 flex items-center justify-center transition-opacity duration-200 ease-out',
          state === 'pending' ? 'opacity-100' : 'pointer-events-none opacity-0',
        )}
      >
        <span className="sr-only">Chargement</span>
        <span className="flex items-center gap-1" aria-hidden="true">
          <span className="size-1.5 animate-bounce rounded-full bg-current [animation-delay:-0.3s]" />
          <span className="size-1.5 animate-bounce rounded-full bg-current [animation-delay:-0.15s]" />
          <span className="size-1.5 animate-bounce rounded-full bg-current" />
        </span>
      </span>
      <span
        className={cn(
          'absolute inset-0 flex items-center justify-center transition-opacity duration-200 ease-out',
          state === 'success' ? 'opacity-100' : 'pointer-events-none opacity-0',
        )}
      >
        <span className="sr-only">Terminé</span>
        <CheckIcon
          key={state}
          className="size-5 animate-in zoom-in-75 duration-300"
          aria-hidden="true"
        />
      </span>
    </Button>
  );
}
