import type { FormEvent } from 'react';
import { Calendar, Check, Mail, MapPin, Pencil, User, X } from 'lucide-react';
import type { AccountPrincipal } from '@nvbes/identity-client';

import {
  Editable,
  EditableArea,
  EditableCancel,
  EditableInput,
  EditableLabel,
  EditablePreview,
  EditableSubmit,
  EditableToolbar,
  EditableTrigger,
} from '@/components/ui/editable';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Separator } from '@/components/ui/separator';
import { cn } from '@/lib/utils';
import { PersonalInfoError, PersonalInfoSuccess } from './AccountPersonalInfoPage.feedback';

function InfoRow({
  icon: Icon,
  label,
  value,
}: {
  icon: typeof User;
  label: string;
  value: string;
}) {
  return (
    <div className="flex min-w-0 items-center gap-3 py-1">
      <div className="flex size-8 shrink-0 items-center justify-center rounded-lg bg-muted">
        <Icon className="size-4 text-muted-foreground" />
      </div>
      <div className="flex min-w-0 flex-col">
        <span className="text-xs text-muted-foreground">{label}</span>
        <span className="truncate text-sm font-medium">{value}</span>
      </div>
    </div>
  );
}

function EditableProfileField({
  className,
  inputType = 'text',
  label,
  onChange,
  placeholder,
  value,
}: {
  className?: string;
  inputType?: 'date' | 'text';
  label: string;
  onChange: (value: string) => void;
  placeholder: string;
  value: string;
}) {
  return (
    <Editable
      value={value}
      onValueChange={onChange}
      placeholder={placeholder}
      className={cn('rounded-lg border bg-background/60 p-3', className)}
    >
      <div className="flex items-start justify-between gap-3">
        <EditableLabel>{label}</EditableLabel>
        <EditableTrigger asChild>
          <Button variant="ghost" size="icon-xs" aria-label={`Modifier ${label}`}>
            <Pencil data-icon="inline-start" />
          </Button>
        </EditableTrigger>
      </div>
      <div className="flex items-center gap-2">
        <EditableArea className="min-w-0 flex-1">
          <EditablePreview className="w-full px-0 font-medium" />
          <EditableInput type={inputType} className="h-8 rounded-lg px-2.5" />
        </EditableArea>
        <EditableToolbar className="shrink-0">
          <EditableSubmit asChild>
            <Button variant="ghost" size="icon-xs" aria-label={`Valider ${label}`}>
              <Check data-icon="inline-start" />
            </Button>
          </EditableSubmit>
          <EditableCancel asChild>
            <Button variant="ghost" size="icon-xs" aria-label={`Annuler ${label}`}>
              <X data-icon="inline-start" />
            </Button>
          </EditableCancel>
        </EditableToolbar>
      </div>
    </Editable>
  );
}

export function PersonalInfoCard({
  user,
  fullName,
  memberSince,
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
  user: AccountPrincipal;
  fullName: string;
  memberSince: string;
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
        <CardTitle>Profil</CardTitle>
        <CardDescription>Vos informations d&apos;identite et de profil.</CardDescription>
      </CardHeader>
      <CardContent>
        <form onSubmit={onSubmit} className="flex flex-col gap-5">
          <div className="grid gap-3 lg:grid-cols-2">
            <InfoRow icon={User} label="Nom complet" value={fullName} />
            <InfoRow icon={Mail} label="Email" value={user.email} />
            <InfoRow icon={MapPin} label="Region active" value={user.region ?? 'Non definie'} />
            <InfoRow icon={Calendar} label="Membre depuis" value={memberSince} />
          </div>

          <div className="flex items-center justify-between gap-3 rounded-lg bg-muted/50 px-3 py-2">
            <span className="text-sm font-medium text-muted-foreground">Statut email</span>
            <Badge variant={user.email_verified ? 'default' : 'secondary'}>
              {user.email_verified ? 'Verifie' : 'En attente'}
            </Badge>
          </div>

          <Separator />

          <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
            <EditableProfileField
              label="Prenom"
              placeholder="Prenom"
              value={firstname}
              onChange={onFirstnameChange}
            />
            <EditableProfileField
              label="Nom"
              placeholder="Nom"
              value={lastname}
              onChange={onLastnameChange}
            />
          </div>

          <EditableProfileField
            label="Nom d'utilisateur"
            placeholder="nomutilisateur"
            value={username}
            onChange={onUsernameChange}
          />

          <EditableProfileField
            label="Date de naissance"
            inputType="date"
            placeholder="Date de naissance"
            value={birthdate}
            onChange={onBirthdateChange}
          />

          <EditableProfileField
            label="Region"
            placeholder="FR"
            value={region}
            onChange={onRegionChange}
          />

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
