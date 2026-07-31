const DEFAULT_STORAGE_KEY = 'nvbes.identity.oauth.transaction';

export interface OAuthTransaction {
  state: string;
  codeVerifier: string;
  nonce: string | null;
  createdAt: number;
  returnTo: string;
}

export interface WebStorage {
  saveTransaction(transaction: OAuthTransaction): void;
  getTransaction(): OAuthTransaction | null;
  clearTransaction(): void;
}

export class MemoryStorage implements WebStorage {
  private transaction: OAuthTransaction | null = null;

  saveTransaction(transaction: OAuthTransaction): void {
    this.transaction = { ...transaction };
  }

  getTransaction(): OAuthTransaction | null {
    return this.transaction ? { ...this.transaction } : null;
  }

  clearTransaction(): void {
    this.transaction = null;
  }
}

export const memoryStorage = new MemoryStorage();

export class BrowserSessionStorage implements WebStorage {
  constructor(
    private readonly storage: Storage,
    private readonly key = DEFAULT_STORAGE_KEY,
  ) {}

  saveTransaction(transaction: OAuthTransaction): void {
    this.storage.setItem(this.key, JSON.stringify(transaction));
  }

  getTransaction(): OAuthTransaction | null {
    const raw = this.storage.getItem(this.key);
    if (!raw) {
      return null;
    }

    try {
      const value: unknown = JSON.parse(raw);
      if (isOAuthTransaction(value)) {
        return value;
      }
    } catch {
      // Invalid browser state is treated as an absent one-time transaction.
    }

    this.clearTransaction();
    return null;
  }

  clearTransaction(): void {
    this.storage.removeItem(this.key);
  }
}

export function defaultWebStorage(): WebStorage {
  if (typeof window === 'undefined') {
    return new MemoryStorage();
  }

  try {
    const probe = `${DEFAULT_STORAGE_KEY}.probe`;
    window.sessionStorage.setItem(probe, '1');
    window.sessionStorage.removeItem(probe);
    return new BrowserSessionStorage(window.sessionStorage);
  } catch {
    return new MemoryStorage();
  }
}

function isOAuthTransaction(value: unknown): value is OAuthTransaction {
  if (!isRecord(value)) {
    return false;
  }

  return (
    typeof value.state === 'string' &&
    value.state.length > 0 &&
    typeof value.codeVerifier === 'string' &&
    value.codeVerifier.length > 0 &&
    (typeof value.nonce === 'string' || value.nonce === null) &&
    typeof value.createdAt === 'number' &&
    Number.isFinite(value.createdAt) &&
    typeof value.returnTo === 'string' &&
    value.returnTo.startsWith('/') &&
    !value.returnTo.startsWith('//')
  );
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}
