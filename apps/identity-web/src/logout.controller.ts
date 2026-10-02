import {
  loadHostedLogoutContext,
  logoutHostedSession,
  prepareHostedRpLogout,
  confirmHostedRpLogout,
} from '@nvbes/identity-sdk-web/oauth';

export function logoutGateway(origin: string, request?: string) {
  const config = { baseUrl: origin };
  return {
    load: async () => {
      const csrf = await loadHostedLogoutContext(config);
      if (csrf && request !== undefined) await prepareHostedRpLogout(config, csrf, request);
      return csrf;
    },
    confirm: async (csrf: string) => {
      if (request !== undefined) return confirmHostedRpLogout(config, csrf, request);
      await logoutHostedSession(config, csrf);
    },
  };
}

interface LogoutState {
  stage: 'loading' | 'confirm' | 'leaving' | 'complete' | 'absent' | 'failed' | 'cancelled';
}

export class LogoutController {
  private state: LogoutState = { stage: 'loading' };
  private proof: string | null = null;
  private started = false;
  private disposed = false;
  private listeners = new Set<() => void>();
  constructor(
    private gateway: ReturnType<typeof logoutGateway>,
    private navigate: (url: string) => void = () => {},
  ) {}
  snapshot = () => this.state;
  subscribe = (listener: () => void) => {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  };
  private publish(stage: LogoutState['stage']) {
    if (this.disposed) return;
    this.state = { stage };
    for (const listener of this.listeners) listener();
  }
  async start() {
    if (this.started || this.disposed) return;
    this.started = true;
    try {
      const proof = await this.gateway.load();
      if (this.disposed) return;
      this.proof = proof;
      this.publish(proof ? 'confirm' : 'absent');
    } catch {
      this.publish('failed');
    }
  }
  async confirm() {
    if (this.disposed || this.state.stage !== 'confirm' || !this.proof) return;
    const proof = this.proof;
    this.proof = null;
    this.publish('leaving');
    try {
      const redirect = await this.gateway.confirm(proof);
      if (this.disposed) return;
      this.publish('complete');
      if (redirect) this.navigate(redirect);
    } catch {
      this.publish('failed');
    }
  }
  cancel() {
    if (this.state.stage !== 'confirm') return;
    this.proof = null;
    this.publish('cancelled');
  }
  dispose() {
    this.proof = null;
    this.publish('cancelled');
    this.disposed = true;
    this.listeners.clear();
  }
}
