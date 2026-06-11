export type SharedAccountOption = {
  id: string;
  email: string;
  displayName: string;
  isActive: boolean;
  avatarFallback?: string;
};

const SWITCHER_MENU_MIN_WIDTH = 384;
const SWITCHER_MENU_OFFSET = 4;

type SwitcherMenuAnchorRect = Pick<DOMRect, 'bottom' | 'left' | 'width'>;

export function findActiveAccount(accounts: SharedAccountOption[]): SharedAccountOption | null {
  return accounts.find((account) => account.isActive) ?? accounts[0] ?? null;
}

export function getSwitcherMenuStyle(rect: SwitcherMenuAnchorRect): Record<string, string> {
  return {
    position: 'fixed',
    top: `${rect.bottom + SWITCHER_MENU_OFFSET}px`,
    left: `${rect.left}px`,
    width: `${Math.max(rect.width, SWITCHER_MENU_MIN_WIDTH)}px`,
  };
}

export function initialsForDisplayName(name: string): string {
  return name
    .trim()
    .split(/\s+/u)
    .filter(Boolean)
    .slice(0, 2)
    .map((part) => part[0]?.toUpperCase() ?? '')
    .join('');
}
