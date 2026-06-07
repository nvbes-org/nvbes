import type { TokenResponse } from './index';

export interface TokenSession {
  userId: string;
  email: string;
  name: string;
  token: TokenResponse;
}

export interface TokenStorage {
  saveSession(session: TokenSession): Promise<void>;
  getSession(userId: string): Promise<TokenSession | null>;
  removeSession(userId: string): Promise<void>;
  getAllSessions(): Promise<TokenSession[]>;
  getCurrentUserId(): Promise<string | null>;
  setCurrentUserId(userId: string | null): Promise<void>;
  clear(): Promise<void>;
}

// In-Memory Storage (Maximum security, lost on tab close/reload)
export class MemoryTokenStorage implements TokenStorage {
  private sessions = new Map<string, TokenSession>();
  private currentUserId: string | null = null;

  async saveSession(session: TokenSession): Promise<void> {
    this.sessions.set(session.userId, session);
  }

  async getSession(userId: string): Promise<TokenSession | null> {
    return this.sessions.get(userId) ?? null;
  }

  async removeSession(userId: string): Promise<void> {
    this.sessions.delete(userId);
    if (this.currentUserId === userId) {
      this.currentUserId = null;
    }
  }

  async getAllSessions(): Promise<TokenSession[]> {
    return Array.from(this.sessions.values());
  }

  async getCurrentUserId(): Promise<string | null> {
    return this.currentUserId;
  }

  async setCurrentUserId(userId: string | null): Promise<void> {
    this.currentUserId = userId;
  }

  async clear(): Promise<void> {
    this.sessions.clear();
    this.currentUserId = null;
  }
}

// Helper to encrypt/decrypt using AES-GCM via Web Crypto API
class CryptoHelper {
  private static keyCache: CryptoKey | null = null;

  private static async getKey(): Promise<CryptoKey | null> {
    if (CryptoHelper.keyCache) {
      return CryptoHelper.keyCache;
    }
    if (typeof window === 'undefined' || !window.crypto?.subtle) {
      return null;
    }

    // To persist the key across reloads but avoid writing it to localStorage,
    // we can use sessionStorage. Since sessionStorage is isolated per tab,
    // this keeps the key reasonably secure and ephemeral.
    const storageKey = 'nvbes_secure_storage_key';
    let base64Key = sessionStorage.getItem(storageKey);
    let rawKey: Uint8Array;

    if (!base64Key) {
      rawKey = window.crypto.getRandomValues(new Uint8Array(32));
      base64Key = btoa(String.fromCharCode(...rawKey));
      try {
        sessionStorage.setItem(storageKey, base64Key);
      } catch {
        // Fallback to in-memory if sessionStorage is disabled/full
      }
    } else {
      rawKey = Uint8Array.from(
        atob(base64Key)
          .split('')
          .map((c) => c.charCodeAt(0)),
      );
    }

    try {
      const keyData = new ArrayBuffer(rawKey.byteLength);
      new Uint8Array(keyData).set(rawKey);
      CryptoHelper.keyCache = await window.crypto.subtle.importKey(
        'raw',
        keyData,
        { name: 'AES-GCM', length: 256 },
        false,
        ['encrypt', 'decrypt'],
      );
      return CryptoHelper.keyCache;
    } catch {
      return null;
    }
  }

  static async encrypt(plaintext: string): Promise<string> {
    const key = await CryptoHelper.getKey();
    if (!key || typeof window === 'undefined') {
      return plaintext; // Fallback to plain text if Web Crypto is unavailable
    }

    const iv = window.crypto.getRandomValues(new Uint8Array(12));
    const encoded = new TextEncoder().encode(plaintext);
    const ciphertext = await window.crypto.subtle.encrypt({ name: 'AES-GCM', iv }, key, encoded);

    const combined = new Uint8Array(iv.length + ciphertext.byteLength);
    combined.set(iv, 0);
    combined.set(new Uint8Array(ciphertext), iv.length);

    return btoa(String.fromCharCode(...combined));
  }

  static async decrypt(ciphertextBase64: string): Promise<string> {
    const key = await CryptoHelper.getKey();
    if (!key || typeof window === 'undefined') {
      return ciphertextBase64;
    }

    try {
      const combined = new Uint8Array(
        atob(ciphertextBase64)
          .split('')
          .map((c) => c.charCodeAt(0)),
      );

      const iv = combined.slice(0, 12);
      const ciphertext = combined.slice(12);

      const decrypted = await window.crypto.subtle.decrypt(
        { name: 'AES-GCM', iv },
        key,
        ciphertext,
      );

      return new TextDecoder().decode(decrypted);
    } catch (e) {
      console.error('Failed to decrypt storage data', e);
      throw new Error('Decryption failed');
    }
  }
}

// Encrypted Persistent Storage (LocalStorage or SessionStorage)
export class EncryptedWebStorage implements TokenStorage {
  constructor(
    private readonly prefix: string,
    private readonly useLocalStorage = true,
  ) {}

  private get storage(): Storage | null {
    if (typeof window === 'undefined') {
      return null;
    }
    return this.useLocalStorage ? window.localStorage : window.sessionStorage;
  }

  private getSessionKey(userId: string): string {
    return `${this.prefix}_session_${userId}`;
  }

  private getCurrentUserKey(): string {
    return `${this.prefix}_current_user`;
  }

  async saveSession(session: TokenSession): Promise<void> {
    const s = this.storage;
    if (!s) return;

    const plaintext = JSON.stringify(session);
    const ciphertext = await CryptoHelper.encrypt(plaintext);
    s.setItem(this.getSessionKey(session.userId), ciphertext);

    // Save user ID to the list of accounts
    const accounts = await this.getAccountIds();
    if (!accounts.includes(session.userId)) {
      accounts.push(session.userId);
      s.setItem(`${this.prefix}_accounts`, JSON.stringify(accounts));
    }
  }

  async getSession(userId: string): Promise<TokenSession | null> {
    const s = this.storage;
    if (!s) return null;

    const ciphertext = s.getItem(this.getSessionKey(userId));
    if (!ciphertext) return null;

    try {
      const plaintext = await CryptoHelper.decrypt(ciphertext);
      return JSON.parse(plaintext) as TokenSession;
    } catch {
      await this.removeSession(userId);
      return null;
    }
  }

  async removeSession(userId: string): Promise<void> {
    const s = this.storage;
    if (!s) return;

    s.removeItem(this.getSessionKey(userId));

    const accounts = await this.getAccountIds();
    const filtered = accounts.filter((id) => id !== userId);
    s.setItem(`${this.prefix}_accounts`, JSON.stringify(filtered));

    const current = await this.getCurrentUserId();
    if (current === userId) {
      await this.setCurrentUserId(filtered[0] ?? null);
    }
  }

  async getAllSessions(): Promise<TokenSession[]> {
    const accounts = await this.getAccountIds();
    const sessions: TokenSession[] = [];
    for (const userId of accounts) {
      const session = await this.getSession(userId);
      if (session) {
        sessions.push(session);
      }
    }
    return sessions;
  }

  async getCurrentUserId(): Promise<string | null> {
    return this.storage?.getItem(this.getCurrentUserKey()) ?? null;
  }

  async setCurrentUserId(userId: string | null): Promise<void> {
    const s = this.storage;
    if (!s) return;
    if (userId) {
      s.setItem(this.getCurrentUserKey(), userId);
    } else {
      s.removeItem(this.getCurrentUserKey());
    }
  }

  async clear(): Promise<void> {
    const s = this.storage;
    if (!s) return;

    const accounts = await this.getAccountIds();
    for (const userId of accounts) {
      s.removeItem(this.getSessionKey(userId));
    }
    s.removeItem(`${this.prefix}_accounts`);
    s.removeItem(this.getCurrentUserKey());
  }

  private async getAccountIds(): Promise<string[]> {
    const s = this.storage;
    if (!s) return [];
    const raw = s.getItem(`${this.prefix}_accounts`);
    if (!raw) return [];
    try {
      return JSON.parse(raw) as string[];
    } catch {
      return [];
    }
  }
}
