import {
  loadPasswordRecovery,
  requestPasswordRecovery,
  resetRecoveredPassword,
} from '@nvbes/identity-sdk-web/oauth';

export interface PasswordRecoveryGateway {
  context(): Promise<string>;
  request(csrf: string, email: string): Promise<void>;
  reset(csrf: string, token: string, password: string): Promise<void>;
}
export function passwordRecoveryGateway(origin: string): PasswordRecoveryGateway {
  const config = { baseUrl: origin };
  return {
    context: () => loadPasswordRecovery(config),
    request: (csrf, email) => requestPasswordRecovery(config, csrf, email),
    reset: (csrf, token, password) => resetRecoveredPassword(config, csrf, token, password),
  };
}
interface State {
  stage: 'loading' | 'request' | 'reset' | 'sent' | 'complete' | 'closed';
  busy: boolean;
  error: string | null;
}
export class PasswordRecoveryController {
  private state: State = { stage: 'loading', busy: false, error: null };
  private listeners = new Set<() => void>();
  private csrf: string | null = null;
  private live = true;
  private started = false;
  constructor(
    private gateway: PasswordRecoveryGateway,
    private token?: string,
  ) {}
  snapshot = (): State => this.state;
  subscribe = (callback: () => void): (() => void) => {
    this.listeners.add(callback);
    return () => {
      this.listeners.delete(callback);
    };
  };
  private set(state: State) {
    this.state = state;
    for (const listener of this.listeners) listener();
  }
  async start() {
    if (this.started || !this.live) return;
    this.started = true;
    if (this.token === '') {
      this.close('Ce lien de récupération est invalide.');
      return;
    }
    try {
      const csrf = await this.gateway.context();
      if (!this.live) return;
      this.csrf = csrf;
      this.set({ stage: this.token ? 'reset' : 'request', busy: false, error: null });
    } catch {
      if (this.live)
        this.close(
          'La récupération est indisponible. Réessayez plus tard depuis votre application.',
        );
    }
  }
  async request(email: string) {
    if (!this.live || this.state.stage !== 'request' || this.state.busy || !this.csrf) return;
    this.set({ ...this.state, busy: true, error: null });
    try {
      await this.gateway.request(this.csrf, email);
      if (this.live) {
        this.csrf = null;
        this.set({ stage: 'sent', busy: false, error: null });
      }
    } catch {
      if (this.live) this.close('La demande n’a pas pu être confirmée. Réessayez plus tard.');
    }
  }
  async reset(password: string, confirmation: string) {
    if (!this.live || this.state.stage !== 'reset' || this.state.busy || !this.csrf || !this.token)
      return;
    if (password !== confirmation) {
      this.set({ ...this.state, error: 'Les mots de passe ne correspondent pas.' });
      return;
    }
    const token = this.token;
    this.token = undefined;
    this.set({ ...this.state, busy: true, error: null });
    try {
      await this.gateway.reset(this.csrf, token, password);
      if (this.live) {
        this.csrf = null;
        this.set({ stage: 'complete', busy: false, error: null });
      }
    } catch {
      if (this.live)
        this.close(
          'Le changement n’a pas pu être confirmé. Le lien peut être expiré ou déjà utilisé. Revenez à votre application pour vérifier votre accès ou demander un nouveau lien.',
        );
    }
  }
  private close(error: string | null) {
    this.token = undefined;
    this.csrf = null;
    this.set({ stage: 'closed', busy: false, error });
  }
  dispose() {
    this.live = false;
    this.close(null);
  }
}

export function recoveryLink(url: URL): string | undefined {
  if (url.search) return '';
  if (!url.hash) return undefined;
  const fields = new URLSearchParams(url.hash.slice(1));
  const token = fields.get('token');
  return fields.size === 1 && token && /^[A-Za-z0-9_-]{43}$/u.test(token) ? token : '';
}
