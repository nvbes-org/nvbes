import { LogOut, Plus, Users } from 'lucide-react';

import type { DriveAccountMenuAsyncHandler } from './DriveAccountMenu.types';

export function DriveAccountMenuActions({
  sessionCount,
  onAddAccount,
  onLogout,
  onLogoutAll,
}: {
  sessionCount: number;
  onAddAccount: DriveAccountMenuAsyncHandler;
  onLogout: DriveAccountMenuAsyncHandler;
  onLogoutAll: DriveAccountMenuAsyncHandler;
}) {
  return (
    <div className="mt-1 space-y-0.5 border-t border-border/60 pt-1">
      <button
        type="button"
        onClick={() => void onAddAccount()}
        className="flex w-full items-center gap-2 rounded-lg px-3 py-2 text-left text-sm transition-colors hover:bg-muted"
      >
        <Plus className="size-4" />
        <span>Ajouter un compte</span>
      </button>

      <button
        type="button"
        onClick={() => void onLogout()}
        className="flex w-full items-center gap-2 rounded-lg px-3 py-2 text-left text-sm text-destructive transition-colors hover:bg-destructive/10"
      >
        <LogOut className="size-4" />
        <span>Se déconnecter</span>
      </button>

      {sessionCount > 1 ? (
        <button
          type="button"
          onClick={() => void onLogoutAll()}
          className="mt-1 flex w-full items-center gap-2 border-t border-border/40 px-3 py-2 pt-1.5 text-left text-xs text-muted-foreground transition-colors hover:bg-muted"
        >
          <Users className="size-3.5" />
          <span>Déconnecter tous les comptes ({sessionCount})</span>
        </button>
      ) : null}
    </div>
  );
}
