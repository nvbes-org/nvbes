import { useMutation, useQueryClient, useSuspenseQuery } from '@tanstack/react-query';
import { z } from 'zod';
import { identityHttpClient } from '../identity.http';

export type Theme = 'system' | 'light' | 'dark';
export type Language = 'fr' | 'en';

export const themeLabels: Record<Theme, string> = {
  system: 'Systeme',
  light: 'Clair',
  dark: 'Sombre',
};

export const languageLabels: Record<Language, string> = {
  fr: 'Francais',
  en: 'English',
};

const PreferencesSchema = z.object({
  theme: z.string(),
  language: z.string(),
  skip_password: z.boolean().default(false),
});

function applyTheme(theme: Theme) {
  const isDark =
    theme === 'dark' ||
    (theme === 'system' && window.matchMedia('(prefers-color-scheme: dark)').matches);
  document.documentElement.classList.toggle('dark', isDark);
}

function fetchPreferences() {
  return identityHttpClient.request('/auth/me/preferences', PreferencesSchema, {
    method: 'GET',
  });
}

export function useAccountPreferencesPage() {
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

  return {
    theme,
    language,
    isPending: mutation.isPending,
    handleThemeChange: (value: Theme) => {
      applyTheme(value);
      mutation.mutate({ theme: value, language });
    },
    handleLanguageChange: (value: Language) => {
      mutation.mutate({ theme, language: value });
    },
  };
}
