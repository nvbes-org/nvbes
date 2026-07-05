import type { AccountEntry, AccountMe, AccountWorkspace } from '@nvbes/identity-client';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import type { MutableRefObject } from 'react';
import { accountQueryKeys } from '@/account.queries';
import { updateProfile } from './AccountPersonalInfoPage.api';
import type { UpdateProfileInput } from './AccountPersonalInfoPage.shared';
import {
  type AccountPersonalInfoQueryData,
  getAccountPersonalInfoContextUpdate,
  getAccountPersonalInfoQueryKey,
} from './useAccountPersonalInfoPage.shared';

export function useAccountPersonalInfoMutation({
  authuser,
  didInitializeFormRef,
  setBirthdate,
  setEditError,
  setEditSuccess,
  setFirstname,
  setLastname,
  setRegion,
  setUsername,
}: {
  authuser: string;
  didInitializeFormRef: MutableRefObject<boolean>;
  setBirthdate: (value: string) => void;
  setEditError: (value: string | null) => void;
  setEditSuccess: (value: boolean) => void;
  setFirstname: (value: string) => void;
  setLastname: (value: string) => void;
  setRegion: (value: string) => void;
  setUsername: (value: string) => void;
}) {
  const queryClient = useQueryClient();
  const personalInfoQueryKey = getAccountPersonalInfoQueryKey(authuser);

  return useMutation({
    mutationFn: (input: UpdateProfileInput) => updateProfile(input),
    onSuccess: (data) => {
      queryClient.setQueryData<AccountPersonalInfoQueryData>(personalInfoQueryKey, (current) => ({
        user: data.user,
        current_workspace_region: current?.current_workspace_region ?? null,
        current_workspace_id: current?.current_workspace_id ?? null,
        workspaces: current?.workspaces ?? [],
      }));
      queryClient.setQueryData<{
        me: AccountMe | null;
        workspaces: AccountWorkspace[];
        accounts: AccountEntry[];
      }>(accountQueryKeys.context(authuser), getAccountPersonalInfoContextUpdate(data));
      didInitializeFormRef.current = true;
      setFirstname(data.user.firstname ?? '');
      setLastname(data.user.lastname ?? '');
      setUsername(data.user.username ?? '');
      setBirthdate(data.user.birthdate ?? '');
      setRegion(data.user.region ?? '');
      setEditSuccess(true);
      setEditError(null);
      void queryClient.invalidateQueries({ queryKey: accountQueryKeys.all });
    },
    onError: () => {
      setEditError(
        'La mise a jour du profil a echoue. Verifiez les champs ou reessayez plus tard.',
      );
      setEditSuccess(false);
    },
  });
}
