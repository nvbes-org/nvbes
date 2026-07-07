import { queryOptions } from '@tanstack/react-query';
import { fetchDriveMe } from './drive.api';
import { fetchDriveMeWithRefresh } from './drive.auth.functions';

export const driveQueryKeys = {
  all: ['drive'] as const,
  me: (accessToken: string) => ['drive', 'me', accessToken] as const,
  workspaces: (accessToken: string) => ['drive', 'workspaces', accessToken] as const,
  files: (workspaceId: string) => ['drive', 'files', workspaceId] as const,
};

export function driveMeQueryOptions(accessToken: string) {
  return queryOptions({
    queryKey: driveQueryKeys.me(accessToken),
    queryFn: () => fetchDriveMeWithRefresh(accessToken),
    retry: false,
    staleTime: 30 * 1000,
    gcTime: 30 * 60 * 1000,
    refetchOnMount: false,
    refetchOnReconnect: false,
  });
}

export async function loadDriveMe(accessToken: string) {
  return fetchDriveMe(accessToken);
}
