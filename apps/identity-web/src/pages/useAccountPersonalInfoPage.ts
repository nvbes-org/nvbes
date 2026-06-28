import { identityClient } from '@nvbes/identity-client';
import type { AccountPrincipal } from '@nvbes/identity-client';
import { useQuery } from '@tanstack/react-query';
import { useEffect, useMemo, useRef, useState, type FormEvent } from 'react';

import type { SupportedRegion } from '@/identity.auth.api';
import { identityAuthMutationKeys, supportedRegionsQueryFn } from '@/identity.auth.queries';
import { fillPersonalInfoForm } from './useAccountPersonalInfoPage.form';
import { useAccountPersonalInfoMutation } from './useAccountPersonalInfoPage.mutation';
import {
  type AccountPersonalInfoQueryData,
  formatMemberSince,
  getAccountPersonalInfoQueryKey,
} from './useAccountPersonalInfoPage.shared';

const regionNameCollator = new Intl.Collator('fr', {
  sensitivity: 'base',
  numeric: true,
});

function regionSortLabel(region: SupportedRegion): string {
  return region.display_name ?? region.country_code;
}

export function useAccountPersonalInfoPage() {
  const didInitializeFormRef = useRef(false);

  const [firstname, setFirstname] = useState('');
  const [lastname, setLastname] = useState('');
  const [username, setUsername] = useState('');
  const [birthdate, setBirthdate] = useState('');
  const [region, setRegion] = useState('');
  const [editError, setEditError] = useState<string | null>(null);
  const [editSuccess, setEditSuccess] = useState(false);

  const personalInfoQueryKey = getAccountPersonalInfoQueryKey();

  const { data: account } = useQuery({
    queryKey: personalInfoQueryKey,
    queryFn: async ({ signal }): Promise<AccountPersonalInfoQueryData> => {
      const [me, workspaces] = await Promise.all([
        identityClient.getMe({ signal }),
        identityClient.listWorkspaces({ signal }),
      ]);
      return {
        user: me.user,
        current_workspace_region: me.current_workspace_region,
        current_workspace_id: me.current_workspace_id,
        workspaces,
      };
    },
    staleTime: 30 * 60 * 1000,
    gcTime: 30 * 60 * 1000,
    refetchOnWindowFocus: true,
  });

  const supportedRegionsQuery = useQuery({
    queryKey: identityAuthMutationKeys.supportedRegions,
    queryFn: supportedRegionsQueryFn,
    staleTime: 30 * 60 * 1000,
    gcTime: 30 * 60 * 1000,
    refetchOnWindowFocus: false,
  });

  const supportedRegions = useMemo(
    () =>
      [...(supportedRegionsQuery.data ?? [])].sort((left, right) =>
        regionNameCollator.compare(regionSortLabel(left), regionSortLabel(right)),
      ),
    [supportedRegionsQuery.data],
  );

  const selectedUser = account ? accountPersonalInfoUser(account) : null;
  const currentWorkspaceRegion = account ? accountPersonalInfoWorkspaceRegion(account) : null;
  const accountRegion = selectedUser?.region ?? currentWorkspaceRegion ?? '';

  const mutation = useAccountPersonalInfoMutation({
    didInitializeFormRef,
    setBirthdate,
    setEditError,
    setEditSuccess,
    setFirstname,
    setLastname,
    setRegion,
    setUsername,
  });

  useEffect(() => {
    if (!selectedUser || didInitializeFormRef.current) {
      return;
    }

    fillPersonalInfoForm({
      selectedUser: {
        ...selectedUser,
        region: accountRegion || null,
      },
      setBirthdate,
      setFirstname,
      setLastname,
      setRegion,
      setUsername,
    });
    didInitializeFormRef.current = true;
  }, [accountRegion, selectedUser]);

  useEffect(() => {
    if (!region && accountRegion) {
      setRegion(accountRegion);
    }
  }, [accountRegion, region]);

  useEffect(() => {
    if (!region || supportedRegions.length === 0) {
      return;
    }

    const resolvedRegion = resolveRegionCountryCode(region, supportedRegions);
    if (resolvedRegion && resolvedRegion !== region) {
      setRegion(resolvedRegion);
    }
  }, [region, supportedRegions]);

  const handleSubmit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setEditError(null);
    setEditSuccess(false);

    mutation.mutate({
      firstname: firstname.trim() || null,
      lastname: lastname.trim() || null,
      username: username.trim() || null,
      birthdate: birthdate || null,
      region: region || null,
    });
  };

  const memberSince = formatMemberSince(selectedUser?.created_at);

  return {
    birthdate,
    editError,
    editSuccess,
    firstname,
    handleSubmit,
    lastname,
    loading: mutation.isPending,
    memberSince,
    region,
    regionLoading: supportedRegionsQuery.isPending,
    regions: supportedRegions,
    selectedUser,
    setBirthdate,
    setFirstname,
    setLastname,
    setRegion,
    setUsername,
    username,
  };
}

function resolveRegionCountryCode(value: string, regions: SupportedRegion[]): string | null {
  const normalizedValue = value.trim().toLowerCase();
  const region = regions.find(
    (entry) =>
      entry.country_code.toLowerCase() === normalizedValue ||
      entry.data_region.toLowerCase() === normalizedValue,
  );
  return region?.country_code ?? null;
}

function accountPersonalInfoUser(
  account: AccountPersonalInfoQueryData | AccountPrincipal,
): AccountPrincipal {
  return 'user' in account ? account.user : account;
}

function accountPersonalInfoWorkspaceRegion(
  account: AccountPersonalInfoQueryData | AccountPrincipal,
): string | null {
  if (!('user' in account)) {
    return null;
  }

  const currentWorkspace = account.workspaces.find(
    (workspace) => workspace.id === account.current_workspace_id,
  );
  return (
    account.current_workspace_region ??
    currentWorkspace?.data_region ??
    account.workspaces[0]?.data_region ??
    null
  );
}
