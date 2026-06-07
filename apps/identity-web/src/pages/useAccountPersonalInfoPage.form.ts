import type { AccountPrincipal } from '@nvbes/identity-client';

export function fillPersonalInfoForm({
  selectedUser,
  setBirthdate,
  setFirstname,
  setLastname,
  setRegion,
  setUsername,
}: {
  selectedUser: AccountPrincipal;
  setBirthdate: (value: string) => void;
  setFirstname: (value: string) => void;
  setLastname: (value: string) => void;
  setRegion: (value: string) => void;
  setUsername: (value: string) => void;
}) {
  setFirstname(selectedUser.firstname ?? '');
  setLastname(selectedUser.lastname ?? '');
  setUsername(selectedUser.username ?? '');
  setBirthdate(selectedUser.birthdate ?? '');
  setRegion(selectedUser.region ?? '');
}
