import { ChevronRight } from 'lucide-react';
import type { ComponentProps, ComponentType, ReactNode } from 'react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  Item,
  ItemActions,
  ItemContent,
  ItemDescription,
  ItemMedia,
  ItemTitle,
} from '@/components/ui/item';

export function IdentityActionItem({
  icon: Icon,
  iconClassName,
  label,
  description,
  badgeLabel,
  badgeVariant,
  trailing,
  variant = 'ghost',
  onAction,
  disabled,
}: {
  icon?: ComponentType<{ className?: string }>;
  iconClassName?: string;
  label: ReactNode;
  description?: ReactNode;
  badgeLabel?: string;
  badgeVariant?: 'default' | 'secondary';
  trailing?: ReactNode;
  variant?: ComponentProps<typeof Button>['variant'];
  onAction: () => void;
  disabled?: boolean;
}) {
  return (
    <Item asChild className="border-0 p-0">
      <Button
        type="button"
        variant={variant}
        className="h-auto w-full cursor-pointer justify-start whitespace-normal rounded-lg px-2 py-3 text-left disabled:cursor-not-allowed"
        onClick={onAction}
        disabled={disabled}
      >
        {Icon ? (
          <ItemMedia variant="icon" className="size-8 rounded-lg bg-muted">
            <Icon className={iconClassName ?? 'size-4 text-muted-foreground'} />
          </ItemMedia>
        ) : null}
        <ItemContent className="min-w-0">
          <ItemTitle>{label}</ItemTitle>
          {description ? (
            <ItemDescription className="truncate text-xs">{description}</ItemDescription>
          ) : null}
        </ItemContent>
        <ItemActions className="ml-auto shrink-0">
          {badgeLabel ? (
            <Badge variant={badgeVariant} className="shrink-0">
              {badgeLabel}
            </Badge>
          ) : null}
          {trailing ?? <ChevronRight className="size-4" />}
        </ItemActions>
      </Button>
    </Item>
  );
}
