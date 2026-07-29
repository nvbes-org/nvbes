import type { AccountPrincipal } from '@nvbes/identity-client';
import { identityClient } from '@nvbes/identity-client';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { type SubmitEvent, useEffect, useMemo, useRef, useState } from 'react';
import { notifyProfileAvatarUpdated } from '@/account.avatar';
import { useAuthuser } from '@/hooks/useAuthuser';
import type { SupportedRegion } from '@/identity.auth.api';
import { identityAuthMutationKeys, supportedRegionsQueryFn } from '@/identity.auth.queries';
import { deleteProfileAvatar, uploadProfileAvatar } from './AccountPersonalInfoPage.api';
import { fillPersonalInfoForm } from './useAccountPersonalInfoPage.form';
import { useAccountPersonalInfoMutation } from './useAccountPersonalInfoPage.mutation';
import {
  type AccountPersonalInfoQueryData,
  formatMemberSince,
  getAccountPersonalInfoQueryKey,
} from './useAccountPersonalInfoPage.shared';
import { hasPersonalInfoErrors, validatePersonalInfo } from './AccountPersonalInfoPage.validation';

const regionNameCollator = new Intl.Collator('fr', {
  sensitivity: 'base',
  numeric: true,
});

function regionSortLabel(region: SupportedRegion): string {
  return region.display_name ?? region.country_code;
}

export function useAccountPersonalInfoPage() {
  const didInitializeFormRef = useRef(false);
  const initializedUserIdRef = useRef<string | null>(null);
  const authuser = useAuthuser();

  const [firstname, setFirstname] = useState('');
  const [lastname, setLastname] = useState('');
  const [username, setUsername] = useState('');
  const [birthdate, setBirthdate] = useState('');
  const [region, setRegion] = useState('');
  const [editError, setEditError] = useState<string | null>(null);
  const [editSuccess, setEditSuccess] = useState(false);
  const [avatarError, setAvatarError] = useState<string | null>(null);
  const [avatarLoading, setAvatarLoading] = useState(false);
  const [avatarVersion, setAvatarVersion] = useState(() => Date.now());

  const personalInfoQueryKey = getAccountPersonalInfoQueryKey(authuser);
  const queryClient = useQueryClient();

  const { data: account } = useQuery({
    queryKey: personalInfoQueryKey,
    queryFn: async ({ signal }): Promise<AccountPersonalInfoQueryData> => {
      const me = await identityClient.getMe({ signal });
      return {
        user: me.user,
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
  const accountRegion = selectedUser?.region ?? '';
  const isPersonalInfoUnchanged = selectedUser
    ? firstname.trim() === (selectedUser.firstname ?? '').trim() &&
      lastname.trim() === (selectedUser.lastname ?? '').trim() &&
      username.trim() === (selectedUser.username ?? '').trim() &&
      birthdate === (selectedUser.birthdate ?? '') &&
      region === accountRegion
    : true;
  const personalInfoErrors = useMemo(
    () =>
      validatePersonalInfo({
        firstname,
        lastname,
        username,
        birthdate,
        region,
        regions: supportedRegions,
      }),
    [birthdate, firstname, lastname, region, supportedRegions, username],
  );
  const isPersonalInfoInvalid = hasPersonalInfoErrors(personalInfoErrors);

  const mutation = useAccountPersonalInfoMutation({
    authuser,
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
    const selectedUserId = selectedUser?.id ?? null;
    if (initializedUserIdRef.current !== selectedUserId) {
      didInitializeFormRef.current = false;
      initializedUserIdRef.current = selectedUserId;
    }

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

  useEffect(() => {
    if (!editSuccess) {
      return;
    }

    const timeout = window.setTimeout(() => setEditSuccess(false), 1500);
    return () => window.clearTimeout(timeout);
  }, [editSuccess]);

  const handleSubmit = (event: SubmitEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (isPersonalInfoUnchanged || isPersonalInfoInvalid) {
      return;
    }
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

  const handleAvatarChange = async (file: File) => {
    setAvatarLoading(true);
    setAvatarError(null);
    try {
      await uploadProfileAvatar(file);
      notifyProfileAvatarUpdated();
      setAvatarVersion(Date.now());
      void queryClient.invalidateQueries({ queryKey: personalInfoQueryKey });
    } catch {
      setAvatarError(
        'La photo n’a pas pu être enregistrée. Utilisez une image JPEG, PNG, WebP, HEIC ou HEIF de 5 Mo maximum.',
      );
    } finally {
      setAvatarLoading(false);
    }
  };

  const handleAvatarDelete = async () => {
    setAvatarLoading(true);
    setAvatarError(null);
    try {
      await deleteProfileAvatar();
      setAvatarVersion(Date.now());
      notifyProfileAvatarUpdated();
    } catch {
      setAvatarError('La suppression de la photo a échoué. Réessayez plus tard.');
    } finally {
      setAvatarLoading(false);
    }
  };

  const memberSince = formatMemberSince(selectedUser?.created_at);

  return {
    authuser,
    avatarVersion,
    birthdate,
    avatarError,
    avatarLoading,
    editError,
    editSuccess,
    firstname,
    handleSubmit,
    handleAvatarChange,
    handleAvatarDelete,
    lastname,
    loading: mutation.isPending,
    isPersonalInfoUnchanged,
    isPersonalInfoInvalid,
    memberSince,
    region,
    regionLoading: supportedRegionsQuery.isPending,
    regions: supportedRegions,
    personalInfoErrors,
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
