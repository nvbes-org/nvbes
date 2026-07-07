import type { ChangeEvent, FormEvent } from 'react';
import type { AccountPrincipal } from '@nvbes/identity-client';

import type { SupportedRegion } from '@/identity.auth.api';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { PersonalInfoError, PersonalInfoSuccess } from './AccountPersonalInfoPage.feedback';
import { RegionSelect } from './RegisterPage.region';

function ProfileField({
  autoComplete,
  id,
  label,
  onChange,
  placeholder,
  type = 'text',
  value,
}: {
  autoComplete?: string;
  id: string;
  label: string;
  onChange: (value: string) => void;
  placeholder: string;
  type?: 'date' | 'text';
  value: string;
}) {
  return (
    <div className="flex flex-col gap-2">
      <Label htmlFor={id}>{label}</Label>
      <Input
        id={id}
        type={type}
        value={value}
        placeholder={placeholder}
        autoComplete={autoComplete}
        onChange={(event: ChangeEvent<HTMLInputElement>) => onChange(event.target.value)}
      />
    </div>
  );
}

export function PersonalInfoCard({
  user,
  memberSince,
  firstname,
  lastname,
  username,
  birthdate,
  region,
  regionLoading,
  regions,
  editError,
  editSuccess,
  loading,
  onFirstnameChange,
  onLastnameChange,
  onUsernameChange,
  onBirthdateChange,
  onRegionChange,
  onSubmit,
}: {
  user: AccountPrincipal;
  memberSince: string;
  firstname: string;
  lastname: string;
  username: string;
  birthdate: string;
  region: string;
  regionLoading: boolean;
  regions: SupportedRegion[];
  editError: string | null;
  editSuccess: boolean;
  loading: boolean;
  onFirstnameChange: (value: string) => void;
  onLastnameChange: (value: string) => void;
  onUsernameChange: (value: string) => void;
  onBirthdateChange: (value: string) => void;
  onRegionChange: (value: string) => void;
  onSubmit: (event: FormEvent<HTMLFormElement>) => void;
}) {
  return (
    <Card>
      <CardHeader>
        <CardTitle>Informations personnelles</CardTitle>
        <CardDescription>
          {memberSince ? `Compte cree le ${memberSince}` : 'Date de creation indisponible'}
          {' · '}
          {user.email_verified ? 'Email verifie' : 'Email en attente'}
        </CardDescription>
      </CardHeader>
      <CardContent>
        <form onSubmit={onSubmit} className="flex flex-col gap-6">
          <section className="grid gap-4 sm:grid-cols-2">
            <ProfileField
              id="account-firstname"
              label="Prenom"
              value={firstname}
              placeholder="Prenom"
              autoComplete="given-name"
              onChange={onFirstnameChange}
            />
            <ProfileField
              id="account-lastname"
              label="Nom"
              value={lastname}
              placeholder="Nom"
              autoComplete="family-name"
              onChange={onLastnameChange}
            />
            <div className="sm:col-span-2">
              <ProfileField
                id="account-username"
                label="Nom d'utilisateur"
                value={username}
                placeholder="nomutilisateur"
                autoComplete="username"
                onChange={onUsernameChange}
              />
            </div>
          </section>

          <section className="grid gap-4 sm:grid-cols-2">
            <ProfileField
              id="account-birthdate"
              label="Date de naissance"
              type="date"
              value={birthdate}
              placeholder="Date de naissance"
              autoComplete="bday"
              onChange={onBirthdateChange}
            />
            <RegionSelect
              id="account-region"
              detectedRegion={region || null}
              reliability="none"
              loading={regionLoading}
              regions={regions}
              value={region}
              onValueChange={onRegionChange}
            />
          </section>

          {editError && <PersonalInfoError message={editError} />}
          {editSuccess && (
            <PersonalInfoSuccess message="Votre profil a ete mis a jour avec succes." />
          )}

          <div className="flex justify-end">
            <Button type="submit" disabled={loading} className="w-full sm:w-auto">
              {loading ? 'Enregistrement...' : 'Enregistrer'}
            </Button>
          </div>
        </form>
      </CardContent>
    </Card>
  );
}
