import { identityClient } from '@nvbes/identity-client';
import { useQuery } from '@tanstack/react-query';
import { useEffect, useRef, useState, type FormEvent } from 'react';

import { fillPersonalInfoForm } from './useAccountPersonalInfoPage.form';
import { useAccountPersonalInfoMutation } from './useAccountPersonalInfoPage.mutation';
import {
  formatMemberSince,
  getAccountPersonalInfoQueryKey,
  getFullName,
} from './useAccountPersonalInfoPage.shared';

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

  const { data: user } = useQuery({
    queryKey: personalInfoQueryKey,
    queryFn: ({ signal }) => identityClient.getMe({ signal }).then((me) => me.user),
    staleTime: 30 * 60 * 1000,
    gcTime: 30 * 60 * 1000,
    refetchOnWindowFocus: true,
  });

  const selectedUser = user ?? null;

  useEffect(() => {
    if (!selectedUser || didInitializeFormRef.current) {
      return;
    }

    fillPersonalInfoForm({
      selectedUser,
      setBirthdate,
      setFirstname,
      setLastname,
      setRegion,
      setUsername,
    });
    didInitializeFormRef.current = true;
  }, [selectedUser]);

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

  const fullName = getFullName(selectedUser);
  const memberSince = formatMemberSince(selectedUser?.created_at);

  return {
    birthdate,
    editError,
    editSuccess,
    firstname,
    fullName,
    handleSubmit,
    lastname,
    loading: mutation.isPending,
    memberSince,
    region,
    selectedUser,
    setBirthdate,
    setFirstname,
    setLastname,
    setRegion,
    setUsername,
    username,
  };
}
