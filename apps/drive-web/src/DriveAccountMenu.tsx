import { runClientEffect } from '@nvbes/web-runtime';
import { useQueryClient } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import { LogOut, Plus, Trash2, Users } from 'lucide-react';
import type { MouseEvent } from 'react';
import { useState, useTransition } from 'react';
import { Avatar, AvatarFallback } from '@/components/ui/avatar';
import type { DriveMeResponse } from './drive.api';
import { driveQueryKeys } from './drive.queries';
import { clearDriveSession, getSessions, removeSession, setActiveSession } from './drive.session';
import { logoutDriveWorkflow, startDriveLoginWorkflow } from './drive.workflow';

type DriveUser = DriveMeResponse['user'];

function initialsFor(name: string): string {
  return name
    .split(' ')
    .filter(Boolean)
    .slice(0, 2)
    .map((part) => part[0]?.toUpperCase() ?? '')
    .join('');
}

export function DriveAccountMenu({ accessToken, user }: { accessToken: string; user: DriveUser }) {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [_isPending, startTransition] = useTransition();
  const [isOpen, setIsOpen] = useState(false);
  const sessions = getSessions();
  const otherSessions = sessions.filter((session) => session.userId !== user.id);
  const initials = initialsFor(user.display_name);

  async function handleLogout() {
    await runClientEffect(logoutDriveWorkflow(accessToken));
    queryClient.removeQueries({ queryKey: driveQueryKeys.all });
    startTransition(() => {
      void navigate({ to: '/', replace: true });
    });
  }

  function handleSwitchAccount(userId: string) {
    setIsOpen(false);
    queryClient.removeQueries({ queryKey: driveQueryKeys.all });
    setActiveSession(userId);
  }

  async function handleAddAccount() {
    setIsOpen(false);
    await runClientEffect(startDriveLoginWorkflow());
  }

  function handleRemoveAccount(event: MouseEvent, userId: string) {
    event.stopPropagation();
    queryClient.removeQueries({ queryKey: driveQueryKeys.all });
    removeSession(userId);
  }

  async function handleLogoutAll() {
    setIsOpen(false);
    queryClient.removeQueries({ queryKey: driveQueryKeys.all });
    await runClientEffect(logoutDriveWorkflow(accessToken));
    clearDriveSession();
    startTransition(() => {
      void navigate({ to: '/', replace: true });
    });
  }

  return (
    <div className="relative">
      <button
        type="button"
        onClick={() => setIsOpen(!isOpen)}
        className="flex items-center gap-2.5 rounded-full border bg-background p-1.5 pr-3 text-left shadow-sm transition-all hover:bg-muted/50 focus:outline-none focus:ring-2 focus:ring-primary/20"
      >
        <Avatar className="size-7">
          <AvatarFallback className="bg-primary text-primary-foreground text-xs">
            {initials || 'DR'}
          </AvatarFallback>
        </Avatar>
        <div className="hidden text-left sm:block">
          <div className="text-xs font-semibold leading-none">{user.display_name}</div>
          <div className="mt-0.5 text-[10px] font-medium text-muted-foreground leading-none">
            {user.email}
          </div>
        </div>
      </button>

      {isOpen && (
        <>
          <div className="fixed inset-0 z-40 cursor-default" onClick={() => setIsOpen(false)} />
          <div className="absolute right-0 z-50 mt-2 w-80 rounded-xl border border-border/80 bg-popover p-2 text-popover-foreground shadow-lg ring-1 ring-black/5 animate-in fade-in-50 slide-in-from-top-1 duration-100">
            <div className="flex items-center gap-3 p-3">
              <Avatar className="size-10">
                <AvatarFallback className="bg-primary text-primary-foreground text-sm">
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

            {otherSessions.length > 0 && (
              <div className="my-1 border-t border-border/60 pt-1">
                <div className="px-3 py-1.5 text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">
                  Autres comptes connectés
                </div>
                <div className="space-y-0.5">
                  {otherSessions.map((session) => {
                    const otherInitials = initialsFor(session.name);
                    return (
                      <div
                        key={session.userId}
                        onClick={() => handleSwitchAccount(session.userId)}
                        className="group flex cursor-pointer items-center gap-3 rounded-lg px-3 py-2 text-left text-sm transition-colors hover:bg-muted"
                      >
                        <Avatar className="size-8">
                          <AvatarFallback className="bg-muted-foreground/20 text-muted-foreground text-xs font-semibold">
                            {otherInitials || 'AC'}
                          </AvatarFallback>
                        </Avatar>
                        <div className="min-w-0 flex-1">
                          <div className="truncate font-medium">{session.name}</div>
                          <div className="truncate text-xs text-muted-foreground">
                            {session.email}
                          </div>
                        </div>
                        <button
                          type="button"
                          onClick={(event) => handleRemoveAccount(event, session.userId)}
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
            )}

            <div className="mt-1 space-y-0.5 border-t border-border/60 pt-1">
              <button
                type="button"
                onClick={() => void handleAddAccount()}
                className="flex w-full items-center gap-2 rounded-lg px-3 py-2 text-left text-sm transition-colors hover:bg-muted"
              >
                <Plus className="size-4" />
                <span>Ajouter un compte</span>
              </button>

              <button
                type="button"
                onClick={() => void handleLogout()}
                className="flex w-full items-center gap-2 rounded-lg px-3 py-2 text-left text-sm text-destructive transition-colors hover:bg-destructive/10"
              >
                <LogOut className="size-4" />
                <span>Se déconnecter</span>
              </button>

              {sessions.length > 1 && (
                <button
                  type="button"
                  onClick={() => void handleLogoutAll()}
                  className="mt-1 flex w-full items-center gap-2 border-t border-border/40 px-3 py-2 pt-1.5 text-left text-xs text-muted-foreground transition-colors hover:bg-muted"
                >
                  <Users className="size-3.5" />
                  <span>Déconnecter tous les comptes ({sessions.length})</span>
                </button>
              )}
            </div>
          </div>
        </>
      )}
    </div>
  );
}
