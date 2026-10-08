import {
  HostedIdentityError,
  listHostedPasskeys,
  listHostedTotpFactors,
  renameHostedPasskey,
  revokeHostedPasskey,
  revokeHostedTotpFactor,
  type HostedPasskey,
  type HostedTotpFactor,
} from '@nvbes/identity-sdk-web/oauth';

export function factorsGateway(origin: string) {
  const config = { baseUrl: origin };
  return {
    list: async (csrf: string) => {
      const [passkeys, totp] = await Promise.all([
        listHostedPasskeys(config, csrf),
        listHostedTotpFactors(config, csrf),
      ]);
      return { passkeys, totp };
    },
    rename: (csrf: string, id: string, label: string) =>
      renameHostedPasskey(config, csrf, id, label),
    revoke: (csrf: string, kind: 'passkey' | 'totp', id: string) =>
      kind === 'passkey'
        ? revokeHostedPasskey(config, csrf, id)
        : revokeHostedTotpFactor(config, csrf, id),
  };
}

interface FactorsState {
  busy: boolean;
  loaded: boolean;
  passkeys: HostedPasskey[];
  totp: HostedTotpFactor[];
  error: string | null;
  notice: string | null;
}

/** Uses a bounded session proof; server ownership and last-factor rules stay authoritative. */
export class FactorsController {
  private state: FactorsState = {
    busy: false,
    loaded: false,
    passkeys: [],
    totp: [],
    error: null,
    notice: null,
  };
  private listeners = new Set<() => void>();
  private started = false;
  private disposed = false;
  constructor(
    private readonly gateway: ReturnType<typeof factorsGateway>,
    private csrf: string,
    readonly expiresAt: string,
    private readonly close: (revoked: boolean) => void,
  ) {}
  snapshot = () => this.state;
  subscribe = (listener: () => void) => {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  };
  private publish(change: Partial<FactorsState>) {
    this.state = { ...this.state, ...change };
    for (const listener of this.listeners) listener();
  }
  private live() {
    if (this.disposed) return false;
    if (!(Date.parse(this.expiresAt) > Date.now())) {
      this.stop(false);
      return false;
    }
    return true;
  }
  expire() {
    this.live();
  }
  private stop(revoked: boolean) {
    this.dispose();
    this.close(revoked);
  }
  dispose() {
    this.disposed = true;
    this.csrf = '';
    this.publish({ passkeys: [], totp: [], loaded: false });
  }
  async start() {
    if (this.started || !this.live()) return;
    this.started = true;
    this.publish({ busy: true });
    try {
      const factors = await this.gateway.list(this.csrf);
      if (this.live()) this.publish({ ...factors, loaded: true });
    } catch {
      if (!this.disposed) this.stop(false);
    } finally {
      this.publish({ busy: false });
    }
  }
  rename(id: string, label: string) {
    if (!this.state.passkeys.some((factor) => factor.id === id)) return Promise.resolve();
    return this.mutate(async () => {
      await this.gateway.rename(this.csrf, id, label);
      if (this.live())
        this.publish({
          passkeys: this.state.passkeys.map((factor) =>
            factor.id === id ? { ...factor, label } : factor,
          ),
          notice: 'Nom enregistré.',
        });
    });
  }
  revoke(kind: 'passkey' | 'totp', id: string) {
    const factors = kind === 'passkey' ? this.state.passkeys : this.state.totp;
    if (!factors.some((factor) => factor.id === id)) return Promise.resolve();
    if (this.state.passkeys.length + this.state.totp.length <= 1) {
      this.publish({ error: 'Ajoutez une autre méthode avant de supprimer la dernière.' });
      return Promise.resolve();
    }
    return this.mutate(async () => {
      await this.gateway.revoke(this.csrf, kind, id);
      if (!this.disposed) this.stop(true);
    });
  }
  private async mutate(work: () => Promise<void>) {
    if (!this.live() || this.state.busy || !this.state.loaded) return;
    this.publish({ busy: true, error: null, notice: null });
    try {
      await work();
    } catch (error) {
      if (!this.live()) return;
      if (error instanceof HostedIdentityError && [400, 429].includes(error.status))
        this.publish({
          error:
            'Modification refusée. La preuve peut avoir expiré ou ce facteur être indispensable. En cas de demandes répétées, patientez.',
        });
      else this.stop(false);
    } finally {
      this.publish({ busy: false });
    }
  }
}
