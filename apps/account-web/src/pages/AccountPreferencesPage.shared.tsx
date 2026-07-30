import { Moon, Settings, Sun } from 'lucide-react';

import { AccountPage, AccountPageHeader } from '@/components/AccountPage';
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
import { cn } from '@/lib/classnames';
import {
  languageLabels,
  themeLabels,
  type Language,
  type Theme,
} from './useAccountPreferencesPage';

export function AccountPreferencesPageContent({
  theme,
  language,
  isPending,
  onThemeChange,
  onLanguageChange,
}: {
  theme: Theme;
  language: Language;
  isPending: boolean;
  onThemeChange: (value: Theme) => void;
  onLanguageChange: (value: Language) => void;
}) {
  return (
    <AccountPage>
      <AccountPageHeader
        size="section"
        title="Preferences"
        description="Personnalisez votre experience utilisateur."
      />

      <Card>
        <CardHeader>
          <CardTitle>Apparence</CardTitle>
          <CardDescription>Choisissez le theme de l&apos;interface.</CardDescription>
        </CardHeader>
        <CardContent className="flex flex-col gap-4">
          <div className="flex flex-wrap gap-2">
            {(Object.keys(themeLabels) as Theme[]).map((currentTheme) => {
              const Icon =
                currentTheme === 'dark' ? Moon : currentTheme === 'light' ? Sun : Settings;

              return (
                <Button
                  key={currentTheme}
                  variant={theme === currentTheme ? 'default' : 'outline'}
                  size="sm"
                  className={cn('gap-2', theme === currentTheme && 'ring-2 ring-primary/30')}
                  onClick={() => onThemeChange(currentTheme)}
                  disabled={isPending}
                >
                  <Icon className="size-3.5" />
                  {themeLabels[currentTheme]}
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
            onValueChange={(value) => onLanguageChange(value as Language)}
            disabled={isPending}
          >
            <SelectTrigger className="w-full sm:w-60">
              <SelectValue placeholder="Choisir une langue" />
            </SelectTrigger>
            <SelectContent>
              <SelectGroup>
                {(Object.keys(languageLabels) as Language[]).map((currentLanguage) => (
                  <SelectItem key={currentLanguage} value={currentLanguage}>
                    {languageLabels[currentLanguage]}
                  </SelectItem>
                ))}
              </SelectGroup>
            </SelectContent>
          </Select>
        </CardContent>
      </Card>
    </AccountPage>
  );
}
