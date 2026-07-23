import type { AccountPrincipal } from '@nvbes/identity-client';
import { LoaderCircle, Trash2 } from 'lucide-react';
import { type ChangeEvent, type SubmitEvent, useEffect, useState } from 'react';
import { type AsyncButtonState, AsyncStateButton } from '@/components/AsyncStateButton';
import { Avatar, AvatarFallback, AvatarImage } from '@/components/ui/avatar';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import type { SupportedRegion } from '@/identity.auth.api';
import { PersonalInfoError } from './AccountPersonalInfoPage.feedback';
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
  isPersonalInfoUnchanged,
  onFirstnameChange,
  onLastnameChange,
  onUsernameChange,
  onBirthdateChange,
  onRegionChange,
  onSubmit,
  avatarUrl,
  avatarError,
  avatarLoading,
  onAvatarChange,
  onAvatarDelete,
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
  isPersonalInfoUnchanged: boolean;
  onFirstnameChange: (value: string) => void;
  onLastnameChange: (value: string) => void;
  onUsernameChange: (value: string) => void;
  onBirthdateChange: (value: string) => void;
  onRegionChange: (value: string) => void;
  onSubmit: (event: SubmitEvent<HTMLFormElement>) => void;
  avatarUrl?: string;
  avatarError?: string | null;
  avatarLoading?: boolean;
  onAvatarChange?: (file: File) => void;
  onAvatarDelete?: () => void;
}) {
  const [showPending, setShowPending] = useState(false);

  useEffect(() => {
    if (!loading) {
      setShowPending(false);
      return;
    }

    const timeout = window.setTimeout(() => setShowPending(true), 180);
    return () => window.clearTimeout(timeout);
  }, [loading]);

  const submitState = loading && showPending ? 'pending' : editSuccess ? 'success' : 'idle';
  return (
    <Card>
      <CardContent>
        <form onSubmit={onSubmit}>
          <div className="flex flex-col gap-6">
            <section className="flex items-center gap-4">
              <Avatar className="size-16" data-size="xl">
                <AvatarImage src={avatarUrl} alt="Photo de profil" />
                <AvatarFallback>{user.display_name.slice(0, 2).toUpperCase()}</AvatarFallback>
              </Avatar>
              <div className="flex flex-col gap-2">
                <Label htmlFor="account-avatar">Photo de profil</Label>
                <div className="flex items-center gap-2">
                  <Input
                    id="account-avatar"
                    type="file"
                    accept="image/jpeg,image/png,image/webp,image/heic,image/heif,image/heic-sequence,image/heif-sequence,.heic,.heif"
                    disabled={avatarLoading}
                    onChange={(event) => {
                      const file = event.target.files?.[0];
                      if (file) onAvatarChange?.(file);
                      event.target.value = '';
                    }}
                    className="min-w-0 flex-1"
                  />
                  <Button
                    type="button"
                    variant="destructive"
                    size="icon"
                    disabled={avatarLoading}
                    onClick={onAvatarDelete}
                    aria-label="Supprimer la photo de profil"
                  >
                    {avatarLoading ? (
                      <LoaderCircle className="animate-spin" aria-hidden="true" />
                    ) : (
                      <Trash2 aria-hidden="true" />
                    )}
                  </Button>
                </div>
                <p className="text-xs text-muted-foreground">
                  JPEG, PNG, WebP, HEIC ou HEIF · 5 Mo maximum
                </p>
                {avatarError && <PersonalInfoError message={avatarError} />}
              </div>
            </section>
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
          </div>

          {editError && <PersonalInfoError message={editError} />}
          <div className="mt-4">
            <AsyncStateButton
              type="submit"
              disabled={loading || isPersonalInfoUnchanged}
              state={submitState as AsyncButtonState}
              message="Enregistrer"
              className="relative"
            />
          </div>
        </form>
      </CardContent>
    </Card>
  );
}
