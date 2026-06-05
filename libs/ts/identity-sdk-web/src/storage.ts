/**
 * Stockage sécurisé pour le web
 * Ne stocke JAMAIS de tokens sensibles en localStorage/sessionStorage
 * Utilise uniquement les cookies HttpOnly (gérés par le backend)
 */

export interface WebStorage {
  saveCodeVerifier(verifier: string): void;
  getCodeVerifier(): string | null;
  clearCodeVerifier(): void;
  saveState(state: string): void;
  getState(): string | null;
  clearState(): void;
}

class MemoryStorage implements WebStorage {
  private verifier: string | null = null;
  private state: string | null = null;

  saveCodeVerifier(verifier: string): void {
    this.verifier = verifier;
  }

  getCodeVerifier(): string | null {
    return this.verifier;
  }

  clearCodeVerifier(): void {
    this.verifier = null;
  }

  saveState(state: string): void {
    this.state = state;
  }

  getState(): string | null {
    return this.state;
  }

  clearState(): void {
    this.state = null;
  }
}

export const memoryStorage = new MemoryStorage();
