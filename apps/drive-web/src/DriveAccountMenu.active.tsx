import { Avatar, AvatarFallback } from '@/components/ui/avatar';
import type { DriveUser } from './DriveAccountMenu.types';

export function DriveAccountMenuActiveUser({
  initials,
  user,
}: {
  initials: string;
  user: DriveUser;
}) {
  return (
    <div className="flex items-center gap-3 p-3">
      <Avatar className="size-10">
        <AvatarFallback className="bg-primary text-sm text-primary-foreground">
          {initials || 'DR'}
        </AvatarFallback>
      </Avatar>
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-1.5 truncate text-sm font-semibold">
          {user.display_name}
          <span className="inline-flex items-center rounded-full bg-emerald-500/10 px-1.5 py-0.5 text-[10px] font-medium text-emerald-500 ring-1 ring-inset ring-emerald-500/20">
            Actif
          </span>
        </div>
        <div className="truncate text-xs text-muted-foreground">{user.email}</div>
      </div>
    </div>
  );
}
