import { createDpopProof } from './dpop';
import { verifyIdToken } from './oidc.id-token';
import {
  parseTokenResponse,
  type AuthorizationCodeTokenResponse,
} from './oauth.authorization-code';

export interface OAuthSessionConfig {
  baseUrl: string;
  clientId: string;
  fetchImpl?: typeof fetch;
}

/** One owner per token family. Tokens and the non-exportable key stay in memory. */
export class OAuthSession {
  private tokens: AuthorizationCodeTokenResponse | null;
  private rotation: Promise<AuthorizationCodeTokenResponse> | undefined;
  private readonly endpoint: string;
  private readonly clientId: string;
  private readonly fetchImpl: typeof fetch;

  constructor(config: OAuthSessionConfig, tokens: AuthorizationCodeTokenResponse) {
    const url = new URL(config.baseUrl);
    const local =
      url.protocol === 'http:' && ['localhost', '127.0.0.1', '[::1]'].includes(url.hostname);
    if (
      (url.protocol !== 'https:' && !local) ||
      url.username ||
      url.password ||
      url.pathname !== '/' ||
      url.search ||
      url.hash ||
      !config.clientId.trim()
    ) {
      throw new Error('Invalid OAuth session configuration.');
    }
    if (tokens.tokenType !== (tokens.dpopKey ? 'DPoP' : 'Bearer')) {
      throw new Error('OAuth session key does not match token binding.');
    }
    if (
      tokens.identity &&
      (tokens.identity.issuer !== url.origin || tokens.identity.clientId !== config.clientId)
    ) {
      throw new Error('OAuth session identity context mismatch.');
    }
    this.endpoint = `${url.origin}/oauth/token`;
    this.clientId = config.clientId;
    this.fetchImpl = config.fetchImpl ?? fetch.bind(globalThis);
    this.tokens = { ...tokens };
  }

  snapshot(): AuthorizationCodeTokenResponse | null {
    return this.tokens ? { ...this.tokens } : null;
  }

  /** Local disposal only; server logout/revocation remains a separate operation. */
  clear(): void {
    this.tokens = null;
  }

  refresh(): Promise<AuthorizationCodeTokenResponse> {
    if (!this.tokens) return Promise.reject(new Error('OAuth session requires reauthentication.'));
    if (this.rotation) return this.rotation;
    if (!this.tokens.refreshToken)
      return Promise.reject(new Error('OAuth session has no refresh token.'));
    this.rotation = this.rotate(this.tokens).finally(() => {
      this.rotation = undefined;
    });
    return this.rotation;
  }

  private async rotate(
    previous: AuthorizationCodeTokenResponse,
  ): Promise<AuthorizationCodeTokenResponse> {
    try {
      const proof = previous.dpopKey
        ? await createDpopProof(previous.dpopKey, 'POST', this.endpoint)
        : undefined;
      if (this.tokens !== previous) throw new Error('OAuth session was cleared.');
      const response = await this.fetchImpl(this.endpoint, {
        method: 'POST',
        credentials: 'omit',
        redirect: 'error',
        cache: 'no-store',
        signal: AbortSignal.timeout(10_000),
        headers: {
          Accept: 'application/json',
          'Content-Type': 'application/x-www-form-urlencoded',
          ...(proof ? { DPoP: proof } : {}),
        },
        body: new URLSearchParams({
          grant_type: 'refresh_token',
          client_id: this.clientId,
          refresh_token: previous.refreshToken!,
        }),
      });
      if (!response.ok) throw new Error(`OAuth refresh failed (${response.status}).`);
      const payload: unknown = await response.json();
      const next = parseTokenResponse(payload);
      if (
        next.tokenType !== previous.tokenType ||
        !next.refreshToken ||
        next.refreshToken === previous.refreshToken ||
        !sameScopes(next.scope, previous.scope)
      ) {
        throw new Error('Invalid OAuth refresh rotation response.');
      }
      const identity = previous.identity
        ? await verifyIdToken({
            token: next.idToken,
            accessToken: next.accessToken,
            issuer: previous.identity.issuer,
            clientId: this.clientId,
            nonce: previous.identity.nonce,
            fetchImpl: this.fetchImpl,
          })
        : undefined;
      if (
        identity &&
        previous.identity &&
        (identity.subject !== previous.identity.subject ||
          identity.authTime !== previous.identity.authTime)
      )
        throw new Error('OAuth refreshed identity changed.');
      if (this.tokens !== previous) throw new Error('OAuth session was cleared.');
      this.tokens = { ...next, identity, returnTo: previous.returnTo, dpopKey: previous.dpopKey };
      return { ...this.tokens };
    } catch (error) {
      // The server may have committed the rotation even when its response was lost.
      // Never replay the previous refresh secret or keep an ambiguous local family.
      this.clear();
      throw error;
    }
  }
}

function sameScopes(a: string, b: string): boolean {
  const left = a.split(' ').sort();
  const right = b.split(' ').sort();
  return left.length === right.length && left.every((scope, index) => scope === right[index]);
}
