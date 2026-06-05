import { useMutation, useQueryClient, useSuspenseQuery } from '@tanstack/react-query';
import { Moon, Settings, Sun } from 'lucide-react';
import { z } from 'zod';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { cn } from '@/lib/utils';
import { identityHttpClient } from '../identity.http';

type Theme = 'system' | 'light' | 'dark';
type Language = 'fr' | 'en';

const themeLabels: Record<Theme, string> = {
  system: 'Systeme',
  light: 'Clair',
  dark: 'Sombre',
};

const languageLabels: Record<Language, string> = {
  fr: 'Francais',
  en: 'English',
};

const PreferencesSchema = z.object({
  theme: z.string(),
  language: z.string(),
  skip_password: z.boolean().default(false),
});

function applyTheme(t: Theme) {
  const isDark =
    t === 'dark' || (t === 'system' && window.matchMedia('(prefers-color-scheme: dark)').matches);
  document.documentElement.classList.toggle('dark', isDark);
}

function fetchPreferences() {
  return identityHttpClient.request('/auth/me/preferences', PreferencesSchema, {
    method: 'GET',
  });
}

export default function AccountPreferencesPage() {
  const queryClient = useQueryClient();

  const { data } = useSuspenseQuery({
    queryKey: ['preferences'],
    queryFn: fetchPreferences,
    staleTime: 30 * 60 * 1000,
    gcTime: 30 * 60 * 1000,
    refetchOnWindowFocus: false,
  });

  const mutation = useMutation({
    mutationFn: ({ theme, language }: { theme: Theme; language: Language }) =>
      identityHttpClient.request('/auth/me/preferences', PreferencesSchema, {
        method: 'PUT',
        body: {
          theme,
          language,
          skip_password: data.skip_password,
        },
      }),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ['preferences'] });
    },
  });

  const theme = data.theme as Theme;
  const language = data.language as Language;

  const handleThemeChange = (value: Theme) => {
    applyTheme(value);
    mutation.mutate({ theme: value, language });
  };

  const handleLanguageChange = (value: Language) => {
    mutation.mutate({ theme, language: value });
  };

  return (
    <div className="flex flex-col gap-6 animate-fade-slide-up [animation-delay:0ms]">
      <div>
        <h1 className="text-xl font-heading font-semibold">Preferences</h1>
        <p className="text-sm text-muted-foreground mt-1">
          Personnalisez votre experience utilisateur.
        </p>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Apparence</CardTitle>
          <CardDescription>Choisissez le theme de l&apos;interface.</CardDescription>
        </CardHeader>
        <CardContent className="flex flex-col gap-4">
          <div className="flex flex-wrap gap-2">
            {(Object.keys(themeLabels) as Theme[]).map((t) => {
              const Icon = t === 'dark' ? Moon : t === 'light' ? Sun : Settings;
              return (
                <Button
                  key={t}
                  variant={theme === t ? 'default' : 'outline'}
                  size="sm"
                  className={cn('gap-2', theme === t && 'ring-2 ring-primary/30')}
                  onClick={() => handleThemeChange(t)}
                  disabled={mutation.isPending}
                >
                  <Icon className="size-3.5" />
                  {themeLabels[t]}
                </Button>
              );
            })}
          </div>
        </CardContent>
      </Card>

      <Card className="animate-fade-slide-up [animation-delay:100ms]">
        <CardHeader>
          <CardTitle>Langue</CardTitle>
          <CardDescription>Choisissez la langue de l&apos;interface.</CardDescription>
        </CardHeader>
        <CardContent>
          <Select
            value={language}
            onValueChange={(v) => handleLanguageChange(v as Language)}
            disabled={mutation.isPending}
          >
            <SelectTrigger className="w-full sm:w-60">
              <SelectValue placeholder="Choisir une langue" />
            </SelectTrigger>
            <SelectContent>
              <SelectGroup>
                {(Object.keys(languageLabels) as Language[]).map((l) => (
                  <SelectItem key={l} value={l}>
                    {languageLabels[l]}
                  </SelectItem>
                ))}
              </SelectGroup>
            </SelectContent>
          </Select>
        </CardContent>
      </Card>
    </div>
  );
}
