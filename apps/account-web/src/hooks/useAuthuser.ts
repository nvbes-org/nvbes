import { useLocation } from '@tanstack/react-router';
import { readAuthuser } from '@/identity.authuser';

export function useAuthuser(): string {
  return useLocation({
    select: (location) => readAuthuser(location.searchStr, location.pathname),
  });
}
