import { LogOut, Users } from 'lucide-react';

import { Button } from '@/components/ui/button';

type DriveAccountMenuAsyncHandler = () => Promise<void>;

export function DriveAccountMenuActions({
  sessionCount,
  onLogout,
  onLogoutAll,
}: {
  sessionCount: number;
  onLogout: DriveAccountMenuAsyncHandler;
  onLogoutAll: DriveAccountMenuAsyncHandler;
}) {
  return (
    <div className="flex flex-col gap-2">
      <Button
        type="button"
        variant="destructive"
        className="w-full justify-start"
        onClick={() => void onLogout()}
      >
        <LogOut data-icon="inline-start" />
        Se déconnecter
      </Button>

      {sessionCount > 1 ? (
        <Button
          type="button"
          variant="ghost"
          className="w-full justify-start text-muted-foreground"
          onClick={() => void onLogoutAll()}
        >
          <Users data-icon="inline-start" />
          Déconnecter tous les comptes ({sessionCount})
        </Button>
      ) : null}
    </div>
  );
}
