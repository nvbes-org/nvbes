import { useMutation } from '@tanstack/react-query';
import { useState } from 'react';

import { exchangeOAuthPlaygroundCode } from '@/developer.api';
import { Field, PageHeader, buttonClass, formString, inputClass } from './Portal.shared';

async function sha256Base64Url(value: string): Promise<string> {
  const bytes = new TextEncoder().encode(value);
  const digest = await crypto.subtle.digest('SHA-256', bytes);

  return btoa(String.fromCharCode(...new Uint8Array(digest)))
    .replace(/\+/g, '-')
    .replace(/\//g, '_')
    .replace(/=+$/g, '');
}

function randomBase64Url() {
  const bytes = new Uint8Array(32);
  crypto.getRandomValues(bytes);

  return btoa(String.fromCharCode(...bytes))
    .replace(/\+/g, '-')
    .replace(/\//g, '_')
    .replace(/=+$/g, '');
}

export function PortalOAuthPlaygroundPage() {
  const [authorizeUrl, setAuthorizeUrl] = useState('');
  const [codeVerifier, setCodeVerifier] = useState('');
  const [codeChallenge, setCodeChallenge] = useState('');
  const [state, setState] = useState('');
  const [nonce, setNonce] = useState('');
  const exchangeCode = useMutation({ mutationFn: exchangeOAuthPlaygroundCode });

  return (
    <section>
      <PageHeader
        title="OAuth playground"
        body="Generate PKCE inputs, open authorize, then exchange a returned code."
      />
      <form
        className="grid gap-3 rounded-md border border-border bg-card p-4 md:grid-cols-2"
        onSubmit={(event) => {
          event.preventDefault();
          const form = new FormData(event.currentTarget);
          const clientId = formString(form, 'client_id').trim();
          const redirectUri = formString(form, 'redirect_uri').trim();
          const scope = formString(form, 'scope', 'openid profile email').trim();
          const nextState = randomBase64Url();
          const nextNonce = randomBase64Url();
          const verifier = randomBase64Url();
          void sha256Base64Url(verifier).then((challenge) => {
            setState(nextState);
            setNonce(nextNonce);
            setCodeVerifier(verifier);
            setCodeChallenge(challenge);
            const url = new URL('/oauth/authorize', window.location.origin);
            url.searchParams.set('response_type', 'code');
            url.searchParams.set('client_id', clientId);
            url.searchParams.set('redirect_uri', redirectUri);
            url.searchParams.set('scope', scope);
            url.searchParams.set('state', nextState);
            url.searchParams.set('nonce', nextNonce);
            url.searchParams.set('code_challenge', challenge);
            url.searchParams.set('code_challenge_method', 'S256');
            setAuthorizeUrl(url.toString());
          });
        }}
      >
        <Field label="Client ID">
          <input name="client_id" className={inputClass} required />
        </Field>
        <Field label="Redirect URI">
          <input name="redirect_uri" className={inputClass} required />
        </Field>
        <Field label="Scope">
          <input name="scope" className={inputClass} defaultValue="openid profile email" />
        </Field>
        <button type="submit" className={buttonClass}>
          Generate authorize URL
        </button>
      </form>
      <div className="mt-4 grid gap-3 rounded-md border border-border bg-card p-4 text-sm">
        <code className="overflow-auto rounded-md bg-background p-3">
          {authorizeUrl || 'Authorize URL'}
        </code>
        <p>state: {state || '-'}</p>
        <p>nonce: {nonce || '-'}</p>
        <p>code verifier: {codeVerifier || '-'}</p>
        <p>code challenge: {codeChallenge || '-'}</p>
      </div>
      <form
        className="mt-4 grid gap-3 rounded-md border border-border bg-card p-4 md:grid-cols-2"
        onSubmit={(event) => {
          event.preventDefault();
          const form = new FormData(event.currentTarget);
          exchangeCode.mutate({
            client_id: formString(form, 'client_id'),
            code: formString(form, 'code'),
            redirect_uri: formString(form, 'redirect_uri'),
            code_verifier: formString(form, 'code_verifier'),
          });
        }}
      >
        <Field label="Client ID">
          <input name="client_id" className={inputClass} required />
        </Field>
        <Field label="Authorization code">
          <input name="code" className={inputClass} required />
        </Field>
        <Field label="Redirect URI">
          <input name="redirect_uri" className={inputClass} required />
        </Field>
        <Field label="Code verifier">
          <input name="code_verifier" className={inputClass} defaultValue={codeVerifier} required />
        </Field>
        <button type="submit" className={buttonClass} disabled={exchangeCode.isPending}>
          Exchange code
        </button>
      </form>
      {exchangeCode.data ? (
        <pre className="mt-4 overflow-auto rounded-md border border-border bg-card p-4 text-xs">
          {JSON.stringify(exchangeCode.data, null, 2)}
        </pre>
      ) : null}
    </section>
  );
}
