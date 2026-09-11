// Start identity-service:test:browser-fixture, then call this exported probe
// with its printed loopback origin. No API requests are mocked or intercepted.
export default async function verifyIdentityWebauthn(page, origin) {
  if (!/^http:\/\/localhost:[0-9]+$/.test(origin)) {
    throw new Error('Only the isolated loopback fixture is allowed');
  }
  const context = await page.context().browser().newContext();
  try {
    const browserPage = await context.newPage();
    await browserPage.goto(origin);
    const cdp = await context.newCDPSession(browserPage);
    await cdp.send('WebAuthn.enable');
    await cdp.send('WebAuthn.addVirtualAuthenticator', {
      options: {
        protocol: 'ctap2',
        transport: 'internal',
        hasResidentKey: true,
        hasUserVerification: true,
        isUserVerified: true,
        automaticPresenceSimulation: true,
      },
    });
    const result = await browserPage.evaluate(async () => {
      const check = (condition, message) => {
        if (!condition) throw new Error(message);
      };
      const bootstrap = await (await fetch('/__fixture/bootstrap')).json();
      const jsonPost = async (path, body, csrf) => {
        const response = await fetch(path, {
          method: 'POST',
          headers: {
            'content-type': 'application/json',
            'x-csrf-token': csrf,
          },
          body: JSON.stringify(body),
        });
        check(response.ok, `${path}: HTTP ${response.status}`);
        return response.json();
      };
      const base64url = (bytes) =>
        btoa(String.fromCharCode(...bytes))
          .replaceAll('+', '-')
          .replaceAll('/', '_')
          .replaceAll('=', '');
      const verifier = base64url(crypto.getRandomValues(new Uint8Array(32)));
      const challenge = base64url(
        new Uint8Array(await crypto.subtle.digest('SHA-256', new TextEncoder().encode(verifier))),
      );
      const authorize = async () => {
        const query = new URLSearchParams({
          client_id: bootstrap.client_id,
          redirect_uri: `${location.origin}/callback`,
          response_type: 'code',
          scope: 'openid email',
          resource: `${location.origin}/oauth/userinfo`,
          state: crypto.randomUUID(),
          nonce: crypto.randomUUID(),
          code_challenge: challenge,
          code_challenge_method: 'S256',
        });
        const response = await fetch(`/oauth/authorize?${query}`);
        check(response.ok, `authorize: HTTP ${response.status}`);
        return response.json();
      };
      const first = await authorize();
      check(first.needs_login, 'Initial interaction must require login');
      const passwordLogin = await jsonPost(
        '/oauth/authorize/login',
        {
          interaction: first.interaction,
          email: bootstrap.email,
          password: bootstrap.password,
        },
        first.csrf_token,
      );
      check(!document.cookie.includes('nvbes-dev-session'), 'Session cookie exposed to JavaScript');
      const enrollment = await jsonPost(
        '/oauth/session/webauthn/registration/options',
        {},
        passwordLogin.session_csrf_token,
      );
      check(
        enrollment.options.publicKey.authenticatorSelection.residentKey === 'preferred',
        'Missing discoverability preference',
      );
      const credential = await navigator.credentials.create({
        publicKey: PublicKeyCredential.parseCreationOptionsFromJSON(enrollment.options.publicKey),
      });
      check(
        credential.getClientExtensionResults().credProps?.rk === true,
        'Fixture did not create a discoverable credential',
      );
      await jsonPost(
        '/oauth/session/webauthn/registration/finish',
        {
          ceremony_id: enrollment.ceremony_id,
          credential: credential.toJSON(),
          label: 'Chromium fixture passkey',
        },
        passwordLogin.session_csrf_token,
      );
      await jsonPost('/oauth/logout', {}, passwordLogin.session_csrf_token);
      const second = await authorize();
      check(second.needs_login, 'Logout did not end the password session');
      const options = await jsonPost(
        '/oauth/authorize/passkey/options',
        { interaction: second.interaction },
        second.csrf_token,
      );
      check(
        options.options.publicKey.allowCredentials.length === 0,
        'Login disclosed selected credential IDs',
      );
      // Explicit modal authentication; conditional autofill UI is separate.
      const assertion = await navigator.credentials.get({
        publicKey: PublicKeyCredential.parseRequestOptionsFromJSON(options.options.publicKey),
      });
      check(assertion.response.userHandle !== null, 'Browser did not discover the account');
      const loggedIn = await jsonPost(
        '/oauth/authorize/passkey/finish',
        {
          interaction: second.interaction,
          ceremony_id: options.ceremony_id,
          credential: assertion.toJSON(),
        },
        second.csrf_token,
      );
      const callback = await jsonPost(
        '/oauth/authorize/approve',
        { interaction: second.interaction },
        loggedIn.csrf_token,
      );
      check(typeof callback.code === 'string', 'Missing authorization code');
      const response = await fetch('/oauth/token', {
        method: 'POST',
        headers: { 'content-type': 'application/x-www-form-urlencoded' },
        body: new URLSearchParams({
          grant_type: 'authorization_code',
          code: callback.code,
          client_id: bootstrap.client_id,
          redirect_uri: `${location.origin}/callback`,
          code_verifier: verifier,
        }),
      });
      check(response.ok, `token: HTTP ${response.status}`);
      const tokens = await response.json();
      const info = await fetch('/oauth/userinfo', {
        headers: { authorization: `Bearer ${tokens.access_token}` },
      });
      check(info.ok, `userinfo: HTTP ${info.status}`);
      const claims = await info.json();
      check(
        claims.sub === bootstrap.principal &&
          claims.email === bootstrap.email &&
          claims.email_verified === true,
        'Incorrect UserInfo claims',
      );
      await fetch('/__fixture/done', { method: 'POST' });
      return { enrollment: true, passwordlessLogin: true, oauthCodeExchange: true, userinfo: true };
    });
    return { browser: context.browser().version(), ...result };
  } finally {
    await context.close();
  }
}
