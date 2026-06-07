import { Avatar, AvatarFallback } from '@/components/ui/avatar';
import type { DriveAccountMenuButtonProps } from './DriveAccountMenu.types';

export function DriveAccountMenuButton({ initials, user, onToggle }: DriveAccountMenuButtonProps) {
  return (
    <button
      type="button"
      onClick={onToggle}
      className="flex items-center gap-2.5 rounded-full border bg-background p-1.5 pr-3 text-left shadow-sm transition-all hover:bg-muted/50 focus:outline-none focus:ring-2 focus:ring-primary/20"
    >
      <Avatar className="size-7">
        <AvatarFallback className="bg-primary text-xs text-primary-foreground">
          {initials || 'DR'}
        </AvatarFallback>
      </Avatar>
      <div className="hidden text-left sm:block">
        <div className="text-xs font-semibold leading-none">{user.display_name}</div>
        <div className="mt-0.5 text-[10px] font-medium leading-none text-muted-foreground">
          {user.email}
        </div>
      </div>
    </button>
  );
}
