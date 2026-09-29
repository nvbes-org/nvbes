import {
  HostedIdentityError,
  WebauthnBrowserError,
  resumeHostedMfaRecovery,
  completeHostedMfaRecovery,
  cancelHostedMfaRecovery,
  type HostedMfaRecovery,
} from '@nvbes/identity-sdk-web/oauth';

export function recoveryGateway(origin: string) {
  const config = { baseUrl: origin };
  return {
    resume: () => resumeHostedMfaRecovery(config),
    complete: (proof: HostedMfaRecovery, label: string) =>
      completeHostedMfaRecovery(config, proof, label),
    cancel: (proof: HostedMfaRecovery) => cancelHostedMfaRecovery(config, proof),
  };
}

interface RecoveryState {
  stage: 'loading' | 'ready' | 'complete' | 'cancelled' | 'closed';
  busy: boolean;
  expiresAt: string | null;
  error: string | null;
}

/** Isolated from ordinary sessions, OAuth interactions and callback state. */
export class RecoveryController {
  private state: RecoveryState = { stage: 'loading', busy: false, expiresAt: null, error: null };
  private proof: HostedMfaRecovery | null = null;
  private listeners = new Set<() => void>();
  private started = false;
  private disposed = false;
  constructor(private readonly gateway: ReturnType<typeof recoveryGateway>) {}
  snapshot = () => this.state;
  subscribe = (listener: () => void) => {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  };
  private publish(change: Partial<RecoveryState>) {
    this.state = { ...this.state, ...change };
    for (const listener of this.listeners) listener();
  }
  private finish(stage: 'complete' | 'cancelled' | 'closed') {
    this.proof = null;
    this.publish({ stage, expiresAt: null, error: null });
  }
  async start() {
    if (this.started || this.disposed) return;
    this.started = true;
    this.publish({ busy: true });
    try {
      const proof = await this.gateway.resume();
      if (this.disposed) return;
      if (Date.parse(proof.expiresAt) <= Date.now()) throw new Error('Expired recovery');
      this.proof = proof;
      this.publish({ stage: 'ready', expiresAt: proof.expiresAt });
    } catch {
      this.finish('closed');
    } finally {
      this.publish({ busy: false });
    }
  }
  complete(label: string) {
    return this.mutate('complete', (proof) => this.gateway.complete(proof, label));
  }
  cancel() {
    return this.mutate('cancelled', (proof) => this.gateway.cancel(proof));
  }
  private async mutate(
    stage: 'complete' | 'cancelled',
    work: (proof: HostedMfaRecovery) => Promise<unknown>,
  ) {
    if (this.disposed || this.state.busy || this.state.stage !== 'ready' || !this.proof) return;
    this.expire();
    if (!this.proof) return;
    this.publish({ busy: true, error: null });
    try {
      await work(this.proof);
      if (!this.disposed) this.finish(stage);
    } catch (error) {
      if (this.disposed) return;
      this.expire();
      if (!this.proof) return;
      if (
        (error instanceof HostedIdentityError && [400, 429].includes(error.status)) ||
        (error instanceof WebauthnBrowserError &&
          [
            'webauthn_not_allowed',
            'webauthn_timeout',
            'webauthn_not_supported',
            'webauthn_unsupported',
          ].includes(error.code))
      ) {
        this.publish({
          error:
            'La récupération n’a pas abouti. Réessayez avec une passkey compatible ou annulez. En cas de demandes répétées, patientez.',
        });
      } else this.finish('closed');
    } finally {
      this.publish({ busy: false });
    }
  }
  expire() {
    if (this.proof && Date.parse(this.proof.expiresAt) <= Date.now()) this.finish('closed');
  }
  dispose() {
    this.disposed = true;
    this.finish('closed');
  }
}
