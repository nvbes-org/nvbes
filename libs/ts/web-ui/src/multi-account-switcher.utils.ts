export type SharedAccountOption = {
  id: string;
  email: string;
  displayName: string;
  isActive: boolean;
  avatarFallback?: string;
  avatarUrl?: string;
};

const SWITCHER_MENU_MIN_WIDTH = 384;
const SWITCHER_MENU_OFFSET = 4;
const SWITCHER_MENU_VIEWPORT_GUTTER = 8;

type SwitcherMenuAnchorRect = Pick<DOMRect, 'bottom' | 'left' | 'width'>;
type SwitcherMenuStyleOptions = {
  minWidth?: number;
  offset?: number;
};

export function findActiveAccount(accounts: SharedAccountOption[]): SharedAccountOption | null {
  return accounts.find((account) => account.isActive) ?? accounts[0] ?? null;
}

export function getSwitcherMenuStyle(
  rect: SwitcherMenuAnchorRect,
  options: SwitcherMenuStyleOptions = {},
): Record<string, string> {
  const minWidth = options.minWidth ?? SWITCHER_MENU_MIN_WIDTH;
  const offset = options.offset ?? SWITCHER_MENU_OFFSET;
  const width = Math.max(rect.width, minWidth);
  const viewportWidth =
    typeof globalThis.innerWidth === 'number' ? globalThis.innerWidth : Number.POSITIVE_INFINITY;
  const maxLeft = viewportWidth - width - SWITCHER_MENU_VIEWPORT_GUTTER;
  const left = Math.max(SWITCHER_MENU_VIEWPORT_GUTTER, Math.min(rect.left, maxLeft));

  return {
    position: 'fixed',
    top: `${rect.bottom + offset}px`,
    left: `${left}px`,
    width: `${width}px`,
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
