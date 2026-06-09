export type SharedAccountOption = {
  id: string;
  email: string;
  displayName: string;
  isActive: boolean;
  avatarFallback?: string;
};

export function findActiveAccount(accounts: SharedAccountOption[]): SharedAccountOption | null {
  return accounts.find((account) => account.isActive) ?? accounts[0] ?? null;
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
