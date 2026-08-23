import { useCallback, useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { ChevronDown } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { Spinner } from '@/components/ui/spinner';
import { driveQueryKeys } from './drive.queries';
import { switchDriveWorkspace } from './drive.workspace.switch';
import type { DriveWorkspaceView } from './drive.api';

function useDriveWorkspaceSwitcher(accessToken: string) {
  const queryClient = useQueryClient();
  const [switchingId, setSwitchingId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const handleSwitch = useCallback(
    async (workspaceId: string) => {
      setError(null);
      setSwitchingId(workspaceId);
      try {
        await switchDriveWorkspace(accessToken, workspaceId);
        await queryClient.invalidateQueries({ queryKey: driveQueryKeys.me(accessToken) });
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Erreur inattendue');
      } finally {
        setSwitchingId(null);
      }
    },
    [accessToken, queryClient],
  );

  return { switchingId, error, handleSwitch };
}

export function DriveWorkspaceSwitcher({
  accessToken,
  workspaceName,
  workspaceId,
  workspaces,
}: {
  accessToken: string;
  workspaceName: string;
  workspaceId: string | null;
  workspaces: DriveWorkspaceView[];
}) {
  const { switchingId, error, handleSwitch } = useDriveWorkspaceSwitcher(accessToken);

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button
          type="button"
          variant="ghost"
          size="xs"
          className="uppercase tracking-[0.18em] text-muted-foreground hover:text-foreground"
        >
          {workspaceName}
          <ChevronDown data-icon="inline-end" aria-hidden="true" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="start" className="w-56">
        {error ? (
          <div className="px-1.5 py-1 text-xs text-destructive" role="alert">
            {error}
          </div>
        ) : null}
        <DropdownMenuGroup>
          {workspaces.map((ws) => {
            const isActive = ws.id === workspaceId;
            const isSwitching = switchingId === ws.id;

            return (
              <DropdownMenuItem
                key={ws.id}
                disabled={isActive || isSwitching}
                onSelect={(event) => {
                  event.preventDefault();
                  if (!isActive && !isSwitching) {
                    void handleSwitch(ws.id);
                  }
                }}
              >
                <span className="flex-1 truncate">{ws.name}</span>
                {isActive ? (
                  <Badge variant="secondary">Actif</Badge>
                ) : isSwitching ? (
                  <Spinner className="size-3" />
                ) : null}
              </DropdownMenuItem>
            );
          })}
        </DropdownMenuGroup>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
