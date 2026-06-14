import { queryOptions } from '@tanstack/react-query';
import { enterpriseClient } from './enterprise.api';

export const enterpriseQueryKeys = {
  context: ['enterprise', 'context'] as const,
  users: ['enterprise', 'users'] as const,
};

export function enterpriseContextQueryOptions() {
  return queryOptions({
    queryKey: enterpriseQueryKeys.context,
    queryFn: ({ signal }) => enterpriseClient.getEnterpriseContext({ signal }),
  });
}

export function enterpriseUsersQueryOptions() {
  return queryOptions({
    queryKey: enterpriseQueryKeys.users,
    queryFn: ({ signal }) => enterpriseClient.getEnterpriseUsers({ signal }),
  });
}
