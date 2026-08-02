import type { AccountProfile, AccountUpdateProfileInput } from '@nvbes/account-client';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { type FormEvent, useEffect, useState } from 'react';
import { accountClient } from '@/account.client';
import { useAccountAuthenticationRecovery } from '@/account.authentication';
import { accountQueryKeys } from '@/account.queries';
import { AccountPage } from '@/components/AccountPage';
import { AccountPageError, AccountPageLoading } from '@/components/AccountPageState';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';

interface ProfileFormState {
  firstname: string;
  lastname: string;
  username: string;
  birthdate: string;
  region: string;
}

const EMPTY_FORM: ProfileFormState = {
  firstname: '',
  lastname: '',
  username: '',
  birthdate: '',
  region: '',
};

export default function AccountProfilePage() {
  const queryClient = useQueryClient();
  const profileQuery = useQuery({
    queryKey: accountQueryKeys.profile,
    queryFn: ({ signal }) => accountClient.getProfile({ signal }),
  });
  const [form, setForm] = useState<ProfileFormState>(EMPTY_FORM);
  const [saved, setSaved] = useState(false);

  useAccountAuthenticationRecovery(profileQuery.error);

  useEffect(() => {
    if (profileQuery.data) {
      setForm(profileFormFromProfile(profileQuery.data));
    }
  }, [profileQuery.data]);

  const updateProfile = useMutation({
    mutationFn: (input: AccountUpdateProfileInput) => accountClient.updateProfile(input),
    onSuccess: (profile) => {
      queryClient.setQueryData(accountQueryKeys.profile, profile);
      setSaved(true);
    },
  });
  useAccountAuthenticationRecovery(updateProfile.error);

  const submit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setSaved(false);
    updateProfile.mutate({
      firstname: form.firstname.trim() || null,
      lastname: form.lastname.trim() || null,
      username: form.username.trim() || null,
      birthdate: form.birthdate || null,
      region: form.region.trim() || null,
    });
  };

  return (
    <AccountPage
      title="Profil"
      description="Les informations publiques et administratives de votre compte."
    >
      {profileQuery.isPending ? (
        <AccountPageLoading label="Chargement du profil…" />
      ) : profileQuery.error || !profileQuery.data ? (
        <AccountPageError error={profileQuery.error} onRetry={() => void profileQuery.refetch()} />
      ) : (
        <form className="space-y-6" onSubmit={submit}>
          <div className="border-b border-border pb-5">
            <p className="text-xs font-medium uppercase tracking-wider text-muted-foreground">
              Nom affiché
            </p>
            <p className="mt-1 text-base font-semibold text-foreground">
              {profileQuery.data.display_name}
            </p>
          </div>

          <div className="grid gap-5 sm:grid-cols-2">
            <ProfileField
              id="firstname"
              label="Prénom"
              value={form.firstname}
              autoComplete="given-name"
              onChange={(firstname) => setForm((current) => ({ ...current, firstname }))}
            />
            <ProfileField
              id="lastname"
              label="Nom"
              value={form.lastname}
              autoComplete="family-name"
              onChange={(lastname) => setForm((current) => ({ ...current, lastname }))}
            />
            <ProfileField
              id="username"
              label="Nom d’utilisateur"
              value={form.username}
              autoComplete="username"
              maxLength={100}
              onChange={(username) => setForm((current) => ({ ...current, username }))}
            />
            <ProfileField
              id="birthdate"
              label="Date de naissance"
              value={form.birthdate}
              type="date"
              autoComplete="bday"
              max={new Date().toISOString().slice(0, 10)}
              onChange={(birthdate) => setForm((current) => ({ ...current, birthdate }))}
            />
            <ProfileField
              id="region"
              label="Région"
              value={form.region}
              autoComplete="country-name"
              onChange={(region) => setForm((current) => ({ ...current, region }))}
            />
          </div>

          {updateProfile.error ? (
            <Alert variant="destructive">
              <AlertTitle>Modification impossible</AlertTitle>
              <AlertDescription>{updateProfile.error.message}</AlertDescription>
            </Alert>
          ) : null}
          {saved ? (
            <Alert>
              <AlertTitle>Profil enregistré</AlertTitle>
              <AlertDescription>Vos informations ont été mises à jour.</AlertDescription>
            </Alert>
          ) : null}

          <Button type="submit" disabled={updateProfile.isPending}>
            {updateProfile.isPending ? 'Enregistrement…' : 'Enregistrer'}
          </Button>
        </form>
      )}
    </AccountPage>
  );
}

function ProfileField({
  id,
  label,
  value,
  onChange,
  ...inputProps
}: {
  id: string;
  label: string;
  value: string;
  onChange: (value: string) => void;
} & Omit<React.ComponentProps<typeof Input>, 'id' | 'value' | 'onChange'>) {
  return (
    <div className="space-y-2">
      <Label htmlFor={id}>{label}</Label>
      <Input
        {...inputProps}
        id={id}
        value={value}
        onChange={(event) => onChange(event.currentTarget.value)}
      />
    </div>
  );
}

function profileFormFromProfile(profile: AccountProfile): ProfileFormState {
  return {
    firstname: profile.firstname ?? '',
    lastname: profile.lastname ?? '',
    username: profile.username ?? '',
    birthdate: profile.birthdate ?? '',
    region: profile.region ?? '',
  };
}
