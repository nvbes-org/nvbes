import { ChevronRight } from 'lucide-react';
import type { ComponentType } from 'react';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';

export function SecurityActionRow({
  icon: Icon,
  iconColor,
  label,
  description,
  badgeLabel,
  badgeVariant,
  onAction,
  disabled,
}: {
  icon: ComponentType<{ className?: string }>;
  iconColor?: string;
  label: string;
  description: string;
  badgeLabel: string;
  badgeVariant: 'default' | 'secondary';
  onAction: () => void;
  disabled?: boolean;
}) {
  return (
    <Button
      type="button"
      variant="ghost"
      className="h-auto w-full cursor-pointer justify-between gap-3 whitespace-normal rounded-lg px-2 py-3 text-left disabled:cursor-not-allowed"
      onClick={onAction}
      disabled={disabled}
    >
      <div className="flex min-w-0 items-center gap-3">
        <div className="flex size-8 shrink-0 items-center justify-center rounded-lg bg-muted">
          <Icon className={iconColor ?? 'size-4 text-muted-foreground'} />
        </div>
        <div className="flex min-w-0 flex-col">
          <span className="text-sm font-medium">{label}</span>
          <span className="truncate text-xs text-muted-foreground">{description}</span>
        </div>
      </div>
      <div className="flex shrink-0 items-center gap-2">
        <Badge variant={badgeVariant} className="shrink-0">
          {badgeLabel}
        </Badge>
        <ChevronRight className="size-4" />
      </div>
    </Button>
  );
}
