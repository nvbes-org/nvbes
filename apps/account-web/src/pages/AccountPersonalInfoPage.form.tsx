import type { AccountPrincipal } from '@nvbes/identity-client';
import { LoaderCircle, Trash2 } from 'lucide-react';
import { type ChangeEvent, type SubmitEvent, useEffect, useMemo, useState } from 'react';
import { type AsyncButtonState, AsyncStateButton } from '@/components/AsyncStateButton';
import { BirthdateField } from '@/components/BirthdateField';
import { birthdateBounds } from '@/components/birthdate';
import { FeedbackAlert } from '@/components/FeedbackAlert';
import { Avatar, AvatarFallback, AvatarImage } from '@/components/ui/avatar';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { Field, FieldError, FieldLabel } from '@/components/ui/field';
import { Input } from '@/components/ui/input';
import type { SupportedRegion } from '@/identity.auth.api';
import type { PersonalInfoFieldErrors } from './AccountPersonalInfoPage.validation';
import { RegionSelect } from './RegisterPage.region';
import { MAX_USERNAME_LENGTH } from '@/identity.username.policy';

function ProfileField({
  autoComplete,
  id,
  label,
  error,
  maxLength,
  onChange,
  placeholder,
  value,
}: {
  autoComplete?: string;
  id: string;
  label: string;
  error?: string;
  maxLength?: number;
  onChange: (value: string) => void;
  placeholder: string;
  value: string;
}) {
  return (
    <Field>
      <FieldLabel htmlFor={id}>
        {label} <span className="text-destructive">*</span>
      </FieldLabel>
      <Input
        id={id}
        type="text"
        value={value}
        placeholder={placeholder}
        autoComplete={autoComplete}
        required
        maxLength={maxLength}
        aria-invalid={Boolean(error)}
        aria-describedby={error ? `${id}-error` : undefined}
        onChange={(event: ChangeEvent<HTMLInputElement>) => onChange(event.target.value)}
      />
      <FieldError id={`${id}-error`}>{error}</FieldError>
    </Field>
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
  isPersonalInfoInvalid,
  isPersonalInfoUnchanged,
  errors,
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
  isPersonalInfoInvalid: boolean;
  isPersonalInfoUnchanged: boolean;
  errors: PersonalInfoFieldErrors;
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
  const [birthdateInputValid, setBirthdateInputValid] = useState(Boolean(birthdate));
  const { minBirthdate, maxBirthdate } = useMemo(() => birthdateBounds(), []);

  useEffect(() => {
    if (!loading) {
      setShowPending(false);
      return;
    }

    const timeout = window.setTimeout(() => setShowPending(true), 180);
    return () => window.clearTimeout(timeout);
  }, [loading]);

  useEffect(() => {
    if (errors.birthdate) {
      setBirthdateInputValid(false);
    } else if (birthdate) {
      setBirthdateInputValid(true);
    }
  }, [birthdate, errors.birthdate]);

  const submitState = loading && showPending ? 'pending' : editSuccess ? 'success' : 'idle';
  const formInvalid = isPersonalInfoInvalid || !birthdateInputValid;
  return (
    <Card>
      <CardContent>
        <form onSubmit={onSubmit}>
          <div className="flex flex-col gap-6">
            <section className="flex items-center gap-4">
              <Avatar className="size-16" data-size="xl">
                <AvatarImage src={avatarUrl} crossOrigin="anonymous" alt="Photo de profil" />
                <AvatarFallback>{user.display_name.slice(0, 2).toUpperCase()}</AvatarFallback>
              </Avatar>
              <Field>
                <FieldLabel htmlFor="account-avatar">Photo de profil</FieldLabel>
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
                {avatarError && <FeedbackAlert tone="error">{avatarError}</FeedbackAlert>}
              </Field>
            </section>
            <section className="grid gap-4 sm:grid-cols-2">
              <ProfileField
                id="account-firstname"
                label="Prénom"
                error={errors.firstname}
                value={firstname}
                placeholder="Prenom"
                autoComplete="given-name"
                onChange={onFirstnameChange}
              />
              <ProfileField
                id="account-lastname"
                label="Nom"
                error={errors.lastname}
                value={lastname}
                placeholder="Nom"
                autoComplete="family-name"
                onChange={onLastnameChange}
              />
              <div className="sm:col-span-2">
                <ProfileField
                  id="account-username"
                  label="Nom d'utilisateur"
                  error={errors.username}
                  maxLength={MAX_USERNAME_LENGTH}
                  value={username}
                  placeholder="Nom d'utilisateur"
                  autoComplete="username"
                  onChange={onUsernameChange}
                />
              </div>
            </section>

            <section className="grid gap-4 sm:grid-cols-2">
              <BirthdateField
                id="account-birthdate"
                value={birthdate}
                min={minBirthdate}
                max={maxBirthdate}
                required
                externalError={errors.birthdate}
                onValidityChange={setBirthdateInputValid}
                onChange={onBirthdateChange}
              />
              <RegionSelect
                id="account-region"
                detectedRegion={region || null}
                reliability="none"
                loading={regionLoading}
                regions={regions}
                value={region}
                error={errors.region}
                onValueChange={onRegionChange}
              />
            </section>
          </div>

          {editError && <FeedbackAlert tone="error">{editError}</FeedbackAlert>}
          <div className="mt-4">
            <AsyncStateButton
              type="submit"
              disabled={loading || isPersonalInfoUnchanged || formInvalid}
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
