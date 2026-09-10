import type { AccountGateway } from './account.gateway';
import type { AccountProfile } from './account.profile';

export interface AccountState {
  stage: 'loading' | 'checking' | 'ready' | 'leaving' | 'profile' | 'error' | 'closed';
  profile: AccountProfile | null;
  message: string | null;
}

export class AccountController {
  private state: AccountState = { stage: 'loading', profile: null, message: null };
  private listeners = new Set<() => void>();
  private started = false;
  private disposed = false;
  private busy = false;
  private expiryTimer: ReturnType<typeof setTimeout> | undefined;
  constructor(
    private gateway: AccountGateway,
    private navigate: (url: string) => void,
  ) {}
  snapshot = () => this.state;
  subscribe = (listener: () => void) => {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  };
  private publish(state: AccountState) {
    if (this.disposed) return;
    this.state = state;
    for (const listener of this.listeners) listener();
  }
  async start(callback?: URL) {
    if (this.started || this.disposed) return;
    this.started = true;
    try {
      await this.gateway.initialize();
      if (this.disposed) return;
      if (!callback) {
        this.publish({ stage: 'ready', profile: null, message: null });
        return;
      }
      const result = await this.gateway.callback(callback);
      if (this.disposed) return;
      if (result === 'denied') {
        this.publish({ stage: 'ready', profile: null, message: 'Vous avez annulé la connexion.' });
        return;
      }
      this.checkExpiration();
      if (this.disposed) return;
      const profile = await this.gateway.profile();
      this.publish({ stage: 'profile', profile, message: null });
    } catch {
      this.fail();
    }
  }
  checkExpiration() {
    if (this.disposed) return;
    const expiry = this.gateway.expiration();
    if (!expiry) return;
    if (this.expiryTimer) clearTimeout(this.expiryTimer);
    const remaining = expiry - Date.now();
    if (remaining <= 0) {
      this.gateway.clear();
      this.publish({
        stage: 'closed',
        profile: null,
        message: 'Votre accès Account a expiré. Reconnectez-vous pour continuer.',
      });
      this.disposed = true;
      return;
    }
    this.expiryTimer = setTimeout(() => this.checkExpiration(), Math.min(remaining, 2_147_483_647));
  }
  async revalidate() {
    this.checkExpiration();
    if (this.disposed || this.state.stage !== 'profile') return;
    // Hide previously authorized data while the resource server checks the live grant.
    // The stage also coalesces focus + visibility events into one request.
    this.publish({ stage: 'checking', profile: null, message: null });
    try {
      const profile = await this.gateway.profile();
      this.checkExpiration();
      this.publish({ stage: 'profile', profile, message: null });
    } catch {
      this.fail();
    }
  }
  async login() {
    if (this.disposed || this.busy || this.state.stage !== 'ready') return;
    this.busy = true;
    this.publish({ stage: 'leaving', profile: null, message: null });
    try {
      const url = await this.gateway.authorize();
      if (!this.disposed) this.navigate(url);
    } catch {
      this.fail();
    }
  }
  private fail() {
    if (this.expiryTimer) clearTimeout(this.expiryTimer);
    this.gateway.clear();
    this.publish({
      stage: 'error',
      profile: null,
      message: 'La connexion ou la lecture du compte n’a pas abouti. Recommencez depuis Account.',
    });
  }
  close() {
    if (this.expiryTimer) clearTimeout(this.expiryTimer);
    this.gateway.clear();
    this.publish({
      stage: 'closed',
      profile: null,
      message:
        'Account est fermé sur cette page. Votre session Identity et vos autres applications restent ouvertes.',
    });
    this.disposed = true;
  }
  logout() {
    if (this.disposed || this.state.stage !== 'profile') return;
    const url = this.gateway.logoutUrl();
    this.close();
    this.navigate(url);
  }
  dispose() {
    this.close();
    this.disposed = true;
    this.listeners.clear();
  }
}
