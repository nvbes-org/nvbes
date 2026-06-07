import { Trash2 } from 'lucide-react';

import { Avatar, AvatarFallback } from '@/components/ui/avatar';
import { initialsFor } from './useDriveAccountMenu';
import type {
  DriveAccountMenuPopoverProps,
  DriveAccountMenuRemoveHandler,
  DriveAccountMenuSessionHandler,
} from './DriveAccountMenu.types';

export function DriveAccountMenuOtherAccounts({
  sessions,
  onSwitchAccount,
  onRemoveAccount,
}: {
  sessions: DriveAccountMenuPopoverProps['otherSessions'];
  onSwitchAccount: DriveAccountMenuSessionHandler;
  onRemoveAccount: DriveAccountMenuRemoveHandler;
}) {
  if (sessions.length === 0) {
    return null;
  }

  return (
    <div className="my-1 border-t border-border/60 pt-1">
      <div className="px-3 py-1.5 text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">
        Autres comptes connectés
      </div>
      <div className="space-y-0.5">
        {sessions.map((session) => {
          const otherInitials = initialsFor(session.name);

          return (
            <div
              key={session.userId}
              onClick={() => onSwitchAccount(session.userId)}
              className="group flex cursor-pointer items-center gap-3 rounded-lg px-3 py-2 text-left text-sm transition-colors hover:bg-muted"
            >
              <Avatar className="size-8">
                <AvatarFallback className="bg-muted-foreground/20 text-xs font-semibold text-muted-foreground">
                  {otherInitials || 'AC'}
                </AvatarFallback>
              </Avatar>
              <div className="min-w-0 flex-1">
                <div className="truncate font-medium">{session.name}</div>
                <div className="truncate text-xs text-muted-foreground">{session.email}</div>
              </div>
              <button
                type="button"
                onClick={(event) => onRemoveAccount(event, session.userId)}
                className="rounded p-1 text-muted-foreground opacity-0 transition-all hover:bg-destructive/10 hover:text-destructive group-hover:opacity-100"
                title="Déconnecter ce compte"
              >
                <Trash2 className="size-3.5" />
              </button>
            </div>
          );
        })}
      </div>
    </div>
  );
}
