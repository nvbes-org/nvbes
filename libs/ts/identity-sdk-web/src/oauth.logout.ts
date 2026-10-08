import { text } from './hosted.transport';

export interface LogoutSubmission {
  action: string;
  fields: Record<string, string>;
}

function webUrl(value: string): URL {
  const url = new URL(value);
  const local =
    url.protocol === 'http:' && ['localhost', '127.0.0.1', '[::1]'].includes(url.hostname);
  if ((!local && url.protocol !== 'https:') || url.username || url.password || url.hash)
    throw new Error('Invalid logout URL');
  return url;
}

/** Only state and a deadline persist; ID Tokens remain in the POST form body. */
export class BrowserLogoutTransaction {
  constructor(
    private storage: Storage,
    private key = 'nvbes.oauth.logout',
  ) {}
  create(config: {
    baseUrl: string;
    clientId: string;
    idToken: string;
    redirectUri: string;
  }): LogoutSubmission {
    const origin = webUrl(config.baseUrl);
    if (origin.pathname !== '/' || origin.search) throw new Error('Invalid Identity origin');
    const redirect = webUrl(config.redirectUri);
    if (redirect.search) throw new Error('Logout callback must not have a query');
    const state = Array.from(crypto.getRandomValues(new Uint8Array(32)), (b) =>
      b.toString(16).padStart(2, '0'),
    ).join('');
    const fields = {
      client_id: text(config.clientId, 128),
      id_token_hint: text(config.idToken, 16_384),
      post_logout_redirect_uri: redirect.href,
      state,
    };
    this.storage.setItem(
      this.key,
      JSON.stringify({ state, until: Date.now() + 300_000, redirectUri: redirect.href }),
    );
    return { action: `${origin.origin}/oauth/end-session`, fields };
  }
  consume(url: URL): void {
    const raw = this.storage.getItem(this.key);
    this.storage.removeItem(this.key);
    const transaction: unknown = raw === null ? null : JSON.parse(raw);
    if (
      !transaction ||
      typeof transaction !== 'object' ||
      !('state' in transaction) ||
      !('until' in transaction) ||
      !('redirectUri' in transaction) ||
      typeof transaction.state !== 'string' ||
      typeof transaction.until !== 'number' ||
      typeof transaction.redirectUri !== 'string' ||
      Date.now() >= transaction.until ||
      url.hash ||
      url.searchParams.size !== 1 ||
      url.searchParams.get('state') !== transaction.state ||
      `${url.origin}${url.pathname}` !== transaction.redirectUri
    )
      throw new Error('Invalid logout callback');
  }
}

/** Synchronous navigation avoids putting the ID Token in a URL or browser history. */
export function submitLogout(submission: LogoutSubmission): void {
  const form = document.createElement('form');
  form.method = 'POST';
  form.action = webUrl(submission.action).href;
  form.hidden = true;
  for (const [name, value] of Object.entries(submission.fields)) {
    const input = document.createElement('input');
    input.type = 'hidden';
    input.name = name;
    input.value = value;
    form.append(input);
  }
  document.body.append(form);
  try {
    form.submit();
  } finally {
    form.remove();
  }
}
