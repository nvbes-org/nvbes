import type { AccountLanguage, AccountPreferences, AccountTheme } from '@nvbes/account-client';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import { accountClient } from '@/account.client';
import { useAccountAuthenticationRecovery } from '@/account.authentication';
import { accountQueryKeys } from '@/account.queries';
import { AccountPage } from '@/components/AccountPage';
import { AccountPageError, AccountPageLoading } from '@/components/AccountPageState';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { Label } from '@/components/ui/label';

export default function AccountPreferencesPage() {
  const queryClient = useQueryClient();
  const preferencesQuery = useQuery({
    queryKey: accountQueryKeys.preferences,
    queryFn: ({ signal }) => accountClient.getPreferences({ signal }),
  });
  const [draft, setDraft] = useState<AccountPreferences | null>(null);
  const [saved, setSaved] = useState(false);

  useAccountAuthenticationRecovery(preferencesQuery.error);

  useEffect(() => {
    if (preferencesQuery.data) {
      setDraft(preferencesQuery.data);
      applyTheme(preferencesQuery.data.theme);
    }
  }, [preferencesQuery.data]);

  const updatePreferences = useMutation({
    mutationFn: (preferences: AccountPreferences) => accountClient.updatePreferences(preferences),
    onSuccess: (preferences) => {
      queryClient.setQueryData(accountQueryKeys.preferences, preferences);
      setDraft(preferences);
      applyTheme(preferences.theme);
      setSaved(true);
    },
  });
  useAccountAuthenticationRecovery(updatePreferences.error);

  return (
    <AccountPage
      title="Préférences"
      description="Personnalisez la langue et l’apparence de votre espace Account."
    >
      {preferencesQuery.isPending ? (
        <AccountPageLoading label="Chargement des préférences…" />
      ) : preferencesQuery.error || !draft ? (
        <AccountPageError
          error={preferencesQuery.error}
          onRetry={() => void preferencesQuery.refetch()}
        />
      ) : (
        <div className="space-y-6">
          <PreferenceSelect
            id="language"
            label="Langue"
            description="Langue utilisée par les interfaces nvbes."
            value={draft.language}
            options={[
              { value: 'fr', label: 'Français' },
              { value: 'en', label: 'English' },
            ]}
            onChange={(language) => {
              setSaved(false);
              setDraft((current) => (current ? { ...current, language } : current));
            }}
          />
          <PreferenceSelect
            id="theme"
            label="Thème"
            description="Choisissez un thème clair, sombre ou adapté au système."
            value={draft.theme}
            options={[
              { value: 'system', label: 'Système' },
              { value: 'light', label: 'Clair' },
              { value: 'dark', label: 'Sombre' },
            ]}
            onChange={(theme) => {
              setSaved(false);
              setDraft((current) => (current ? { ...current, theme } : current));
              applyTheme(theme);
            }}
          />

          {updatePreferences.error ? (
            <Alert variant="destructive">
              <AlertTitle>Enregistrement impossible</AlertTitle>
              <AlertDescription>{updatePreferences.error.message}</AlertDescription>
            </Alert>
          ) : null}
          {saved ? (
            <Alert>
              <AlertTitle>Préférences enregistrées</AlertTitle>
              <AlertDescription>Vos choix sont désormais actifs.</AlertDescription>
            </Alert>
          ) : null}

          <Button
            type="button"
            disabled={updatePreferences.isPending}
            onClick={() => {
              setSaved(false);
              updatePreferences.mutate(draft);
            }}
          >
            {updatePreferences.isPending ? 'Enregistrement…' : 'Enregistrer'}
          </Button>
        </div>
      )}
    </AccountPage>
  );
}

function PreferenceSelect<T extends AccountLanguage | AccountTheme>({
  id,
  label,
  description,
  value,
  options,
  onChange,
}: {
  id: string;
  label: string;
  description: string;
  value: T;
  options: ReadonlyArray<{ value: T; label: string }>;
  onChange: (value: T) => void;
}) {
  return (
    <div className="flex flex-col gap-3 border-b border-border pb-6 sm:flex-row sm:items-center sm:justify-between">
      <div>
        <Label htmlFor={id}>{label}</Label>
        <p className="mt-1 text-sm text-muted-foreground">{description}</p>
      </div>
      <select
        id={id}
        className="h-9 min-w-40 rounded-lg border border-input bg-card px-3 text-sm outline-none focus-visible:ring-3 focus-visible:ring-ring/50"
        value={value}
        onChange={(event) => onChange(event.currentTarget.value as T)}
      >
        {options.map((option) => (
          <option key={option.value} value={option.value}>
            {option.label}
          </option>
        ))}
      </select>
    </div>
  );
}

function applyTheme(theme: AccountTheme): void {
  if (theme === 'system') {
    document.documentElement.classList.toggle(
      'dark',
      window.matchMedia('(prefers-color-scheme: dark)').matches,
    );
    return;
  }
  document.documentElement.classList.toggle('dark', theme === 'dark');
}
