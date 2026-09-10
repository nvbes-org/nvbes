import {
  HostedIdentityError,
  WebauthnBrowserError,
  type HostedInteraction,
  type HostedAuthenticationStatus,
  type HostedTotpEnrollment,
} from '@nvbes/identity-sdk-web/oauth';
import type { IdentityGateway } from './authorization.gateway';

export interface AuthorizationState {
  stage:
    | 'loading'
    | 'login'
    | 'step-up'
    | 'enrollment'
    | 'recovery-codes'
    | 'consent'
    | 'leaving'
    | 'closed';
  busy: boolean;
  interaction: HostedInteraction | null;
  authentication: HostedAuthenticationStatus | null;
  error: string | null;
  firstEnrollmentAvailable: boolean;
  totpEnrollment: HostedTotpEnrollment | null;
  recoveryCodes: string[] | null;
}

/** One instance per document; never cache credentials, interactions or mutations. */
export class AuthorizationController {
  private state: AuthorizationState = {
    stage: 'loading',
    busy: false,
    interaction: null,
    authentication: null,
    error: null,
    firstEnrollmentAvailable: false,
    totpEnrollment: null,
    recoveryCodes: null,
  };
  private listeners = new Set<() => void>();
  private started = false;
  private disposed = false;

  constructor(
    private readonly gateway: IdentityGateway,
    private readonly navigate: (url: string) => void,
  ) {}

  snapshot = () => this.state;
  subscribe = (listener: () => void) => {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  };

  private publish(change: Partial<AuthorizationState>) {
    this.state = { ...this.state, ...change };
    for (const listener of this.listeners) listener();
  }

  async start(url: string) {
    if (this.started || this.disposed) return;
    this.started = true;
    this.publish({ busy: true });
    try {
      await this.refresh(await this.gateway.load(url));
    } catch {
      this.close();
    } finally {
      this.publish({ busy: false });
    }
  }

  private close() {
    this.publish({
      stage: 'closed',
      interaction: null,
      authentication: null,
      firstEnrollmentAvailable: false,
      totpEnrollment: null,
      recoveryCodes: null,
      error:
        'Cette connexion ne peut pas être poursuivie. Revenez à votre application pour recommencer.',
    });
  }

  dispose() {
    this.disposed = true;
    this.close();
  }

  private async refresh(interaction: HostedInteraction) {
    if (this.disposed) return;
    // Keep rotated CSRF before the next network request.
    this.publish({ interaction, recoveryCodes: null });
    const authentication = await this.gateway.status(interaction);
    if (this.disposed) return;
    const firstEnrollmentAvailable =
      !authentication.needsLogin &&
      authentication.proofExpiresAt === null &&
      interaction.sessionCsrfToken !== null &&
      !(await this.gateway.hasFactors(interaction.sessionCsrfToken));
    if (this.disposed) return;
    this.publish({
      authentication,
      firstEnrollmentAvailable,
      totpEnrollment:
        firstEnrollmentAvailable && authentication.minimumAuthentication !== 'recent_webauthn'
          ? this.state.totpEnrollment
          : null,
      error: null,
      stage: authentication.needsLogin
        ? 'login'
        : firstEnrollmentAvailable &&
            (authentication.needsStepUp || this.state.totpEnrollment !== null)
          ? 'enrollment'
          : authentication.needsStepUp
            ? 'step-up'
            : 'consent',
    });
  }

  private async mutate(work: (interaction: HostedInteraction) => Promise<void>) {
    const { interaction, busy, stage } = this.state;
    if (this.disposed || !interaction || busy || stage === 'closed' || stage === 'leaving') return;
    this.publish({ busy: true, error: null });
    try {
      await work(interaction);
    } catch (error) {
      if (this.disposed) return;
      // Known refusal or browser cancellation is retryable, but never retry a mutation automatically.
      if (
        (error instanceof HostedIdentityError && [400, 401, 429].includes(error.status)) ||
        (error instanceof WebauthnBrowserError &&
          [
            'webauthn_not_allowed',
            'webauthn_timeout',
            'webauthn_not_supported',
            'webauthn_unsupported',
          ].includes(error.code))
      ) {
        try {
          await this.refresh(this.state.interaction ?? interaction);
          this.publish({
            error:
              'La vérification n’a pas abouti. Vérifiez vos informations ou utilisez une autre méthode. En cas de demandes répétées, patientez avant de réessayer.',
          });
        } catch {
          this.close();
        }
      } else {
        // A lost response can hide a committed mutation; require a new authorization.
        this.close();
      }
    } finally {
      this.publish({ busy: false });
    }
  }

  password(email: string, password: string) {
    if (this.state.stage !== 'login') return Promise.resolve();
    return this.mutate(async (interaction) => {
      await this.refresh(await this.gateway.password(interaction, email, password));
    });
  }

  passkey() {
    if (this.state.stage !== 'login' && this.state.stage !== 'step-up') return Promise.resolve();
    return this.mutate(async (interaction) => {
      if (this.state.stage === 'login') {
        await this.refresh(await this.gateway.passkey(interaction));
      } else {
        if (!interaction.sessionCsrfToken) throw new Error('Missing session proof');
        await this.gateway.stepUpPasskey(interaction.sessionCsrfToken);
        await this.refresh(interaction);
      }
    });
  }

  totp(code: string) {
    if (
      this.state.stage !== 'step-up' ||
      this.state.authentication?.minimumAuthentication !== 'recent_mfa'
    )
      return Promise.resolve();
    return this.mutate(async (interaction) => {
      if (!interaction.sessionCsrfToken) throw new Error('Missing session proof');
      await this.gateway.stepUpTotp(interaction.sessionCsrfToken, code);
      await this.refresh(interaction);
    });
  }

  consent(decision: 'approve' | 'deny') {
    if (decision === 'approve' && this.state.stage !== 'consent') return Promise.resolve();
    return this.mutate(async (interaction) => {
      const destination = await this.gateway.consent(interaction, decision);
      if (this.disposed) return;
      this.publish({
        stage: 'leaving',
        interaction: null,
        authentication: null,
        totpEnrollment: null,
        recoveryCodes: null,
        firstEnrollmentAvailable: false,
      });
      this.navigate(destination);
    });
  }

  beginEnrollment() {
    if (!this.state.busy && this.state.firstEnrollmentAvailable && this.state.stage === 'consent')
      this.publish({ stage: 'enrollment', error: null });
  }

  generateRecoveryCodes() {
    const expiry = this.state.authentication?.proofExpiresAt;
    if (this.state.stage !== 'consent' || !expiry || Date.parse(expiry) <= Date.now())
      return Promise.resolve();
    return this.mutate(async (interaction) => {
      if (!interaction.sessionCsrfToken) throw new Error('Missing session proof');
      const codes = await this.gateway.recoveryCodes(interaction.sessionCsrfToken);
      if (this.disposed) return;
      if (Date.parse(expiry) <= Date.now()) {
        this.close();
        return;
      }
      this.publish({ stage: 'recovery-codes', recoveryCodes: codes });
    });
  }

  dismissRecoveryCodes() {
    if (this.state.stage !== 'recovery-codes') return Promise.resolve();
    return this.mutate(async (interaction) => {
      this.publish({ recoveryCodes: null });
      await this.refresh(interaction);
    });
  }

  expireRecoveryCodes() {
    const expiry = this.state.authentication?.proofExpiresAt;
    if (this.state.recoveryCodes && (!expiry || Date.parse(expiry) <= Date.now())) this.close();
  }

  cancelEnrollment() {
    if (this.state.stage !== 'enrollment') return Promise.resolve();
    return this.mutate(async (interaction) => {
      this.publish({ totpEnrollment: null });
      await this.refresh(interaction);
    });
  }

  registerPasskey(label: string) {
    if (this.state.stage !== 'enrollment' || !this.state.firstEnrollmentAvailable)
      return Promise.resolve();
    return this.mutate(async (interaction) => {
      if (!interaction.sessionCsrfToken) throw new Error('Missing session proof');
      this.publish({ totpEnrollment: null });
      await this.gateway.registerPasskey(interaction.sessionCsrfToken, label);
      if (this.disposed) return;
      // Enrollment does not itself assert fresh WebAuthn authentication.
      await this.gateway.stepUpPasskey(interaction.sessionCsrfToken);
      await this.refresh(interaction);
    });
  }

  startTotp() {
    if (
      this.state.stage !== 'enrollment' ||
      !this.state.firstEnrollmentAvailable ||
      this.state.authentication?.minimumAuthentication === 'recent_webauthn' ||
      this.state.totpEnrollment
    )
      return Promise.resolve();
    return this.mutate(async (interaction) => {
      if (!interaction.sessionCsrfToken) throw new Error('Missing session proof');
      const enrollment = await this.gateway.startTotp(interaction.sessionCsrfToken);
      if (this.disposed) return;
      if (Date.parse(enrollment.expiresAt) <= Date.now()) throw new Error('Enrollment expired');
      this.publish({ totpEnrollment: enrollment });
    });
  }

  confirmTotp(code: string) {
    const enrollment = this.state.totpEnrollment;
    if (
      this.state.stage !== 'enrollment' ||
      !enrollment ||
      this.state.authentication?.minimumAuthentication === 'recent_webauthn'
    )
      return Promise.resolve();
    return this.mutate(async (interaction) => {
      if (!interaction.sessionCsrfToken) throw new Error('Missing session proof');
      if (Date.parse(enrollment.expiresAt) <= Date.now()) {
        this.expireTotp();
        return;
      }
      await this.gateway.confirmTotp(interaction.sessionCsrfToken, enrollment.factorId, code);
      this.publish({ totpEnrollment: null });
      await this.refresh(interaction);
    });
  }

  expireTotp() {
    if (this.state.totpEnrollment && Date.parse(this.state.totpEnrollment.expiresAt) <= Date.now())
      this.publish({
        totpEnrollment: null,
        error: 'Cette configuration a expiré. Commencez une nouvelle configuration.',
      });
  }
}
