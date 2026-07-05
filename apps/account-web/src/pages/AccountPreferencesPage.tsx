import { AccountPreferencesPageContent } from './AccountPreferencesPage.shared';
import { useAccountPreferencesPage } from './useAccountPreferencesPage';

export default function AccountPreferencesPage() {
  const page = useAccountPreferencesPage();

  return (
    <AccountPreferencesPageContent
      {...page}
      onThemeChange={page.handleThemeChange}
      onLanguageChange={page.handleLanguageChange}
    />
  );
}
