import { AlertCircle, Info, X } from 'lucide-react';
import { cn } from '@/lib/classnames';
import { Button } from './button';
import type { ToastRecord } from './toast.shared';

export function ToastViewport({
  toasts,
  onDismiss,
}: {
  toasts: ToastRecord[];
  onDismiss: (id: string) => void;
}) {
  if (toasts.length === 0) {
    return null;
  }

  return (
    <div className="pointer-events-none fixed top-4 right-4 z-50 flex w-[calc(100vw-2rem)] max-w-sm flex-col gap-3 sm:top-6 sm:right-6">
      {toasts.map((toastItem) => (
        <ToastCard key={toastItem.id} toast={toastItem} onDismiss={onDismiss} />
      ))}
    </div>
  );
}

function ToastCard({ toast, onDismiss }: { toast: ToastRecord; onDismiss: (id: string) => void }) {
  const Icon = toast.variant === 'destructive' ? AlertCircle : Info;

  return (
    <div
      role="status"
      aria-live={toast.variant === 'destructive' ? 'assertive' : 'polite'}
      className={cn(
        'pointer-events-auto rounded-xl border bg-background/95 p-4 shadow-lg shadow-black/5 backdrop-blur',
        toast.variant === 'destructive'
          ? 'border-destructive/20 text-destructive dark:border-destructive/30'
          : 'border-border text-foreground',
      )}
    >
      <div className="flex gap-3">
        <div
          className={cn(
            'mt-0.5 flex size-8 shrink-0 items-center justify-center rounded-full',
            toast.variant === 'destructive'
              ? 'bg-destructive/10 text-destructive'
              : 'bg-muted text-muted-foreground',
          )}
        >
          <Icon className="size-4" aria-hidden="true" />
        </div>

        <div className="min-w-0 flex-1">
          <p className="text-sm font-medium leading-5">{toast.title}</p>
          {toast.description ? (
            <p className="mt-1 text-sm leading-5 text-muted-foreground">{toast.description}</p>
          ) : null}
        </div>

        <Button
          type="button"
          size="icon-sm"
          variant="ghost"
          className="-mr-1 -mt-1 size-7 shrink-0"
          onClick={() => onDismiss(toast.id)}
          aria-label="Fermer la notification"
        >
          <X className="size-4" aria-hidden="true" />
        </Button>
      </div>
    </div>
  );
}
