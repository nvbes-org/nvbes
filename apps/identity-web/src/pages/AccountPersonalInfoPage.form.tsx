import type { FormEvent } from 'react';

import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { PersonalInfoError, PersonalInfoSuccess } from './AccountPersonalInfoPage.feedback';

export function PersonalInfoForm({
  firstname,
  lastname,
  username,
  birthdate,
  region,
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
  firstname: string;
  lastname: string;
  username: string;
  birthdate: string;
  region: string;
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
        <CardTitle>Modifier le profil</CardTitle>
        <CardDescription>Mettez a jour vos informations personnelles.</CardDescription>
      </CardHeader>
      <CardContent>
        <form onSubmit={onSubmit} className="flex flex-col gap-5">
          <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
            <div className="flex flex-col gap-2">
              <Label htmlFor="firstname">Prenom</Label>
              <Input
                id="firstname"
                placeholder="Prenom"
                value={firstname}
                onChange={(event) => onFirstnameChange(event.target.value)}
              />
            </div>
            <div className="flex flex-col gap-2">
              <Label htmlFor="lastname">Nom</Label>
              <Input
                id="lastname"
                placeholder="Nom"
                value={lastname}
                onChange={(event) => onLastnameChange(event.target.value)}
              />
            </div>
          </div>

          <div className="flex flex-col gap-2">
            <Label htmlFor="username">Nom d&apos;utilisateur</Label>
            <Input
              id="username"
              placeholder="nomutilisateur"
              value={username}
              onChange={(event) => onUsernameChange(event.target.value)}
            />
          </div>

          <div className="flex flex-col gap-2">
            <Label htmlFor="birthdate">Date de naissance</Label>
            <Input
              id="birthdate"
              type="date"
              value={birthdate}
              onChange={(event) => onBirthdateChange(event.target.value)}
            />
          </div>

          <div className="flex flex-col gap-2">
            <Label htmlFor="region">Region</Label>
            <Input
              id="region"
              placeholder="FR"
              value={region}
              onChange={(event) => onRegionChange(event.target.value)}
            />
          </div>

          {editError && <PersonalInfoError message={editError} />}
          {editSuccess && (
            <PersonalInfoSuccess message="Votre profil a ete mis a jour avec succes." />
          )}

          <Button type="submit" disabled={loading} className="w-full">
            {loading ? 'Enregistrement...' : 'Enregistrer les modifications'}
          </Button>
        </form>
      </CardContent>
    </Card>
  );
}
