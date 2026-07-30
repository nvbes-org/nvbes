import type { ComponentProps, ComponentType } from 'react';
import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';

export interface MfaMethodChoice<T extends string> {
  icon: ComponentType<{ className?: string }>;
  label: string;
  value: T;
  variant?: ComponentProps<typeof Button>['variant'];
}

export function MfaMethodChoiceList<T extends string>({
  choices,
  emptyMessage,
  buttonClassName,
  onSelect,
}: {
  choices: MfaMethodChoice<T>[];
  emptyMessage?: string;
  buttonClassName?: string;
  onSelect: (value: T) => void;
}) {
  if (choices.length === 0 && emptyMessage) {
    return <p className="py-4 text-center text-sm text-muted-foreground">{emptyMessage}</p>;
  }

  return (
    <div className="flex flex-col gap-2">
      {choices.map(({ icon: Icon, label, value, variant = 'outline' }) => (
        <Button
          key={value}
          type="button"
          variant={variant}
          className={cn('justify-start gap-3', buttonClassName)}
          onClick={() => onSelect(value)}
        >
          <Icon className={variant === 'default' ? 'size-4' : 'size-4 text-muted-foreground'} />
          {label}
        </Button>
      ))}
    </div>
  );
}
