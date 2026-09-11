// Exported Playwright probe. Pass the function source (without `export default`)
// to browser_run_code's code input, or import it in a Playwright test runner.
// This isolates browser option semantics; it does not call the Identity API.
export default async function probeWebauthnResidency(page) {
  const context = await page.context().browser().newContext();
  try {
    const probe = await context.newPage();
    await probe.route('http://localhost:9876/webauthn-proof', (route) =>
      route.fulfill({
        contentType: 'text/html',
        body: '<!doctype html><title>WebAuthn probe</title>',
      }),
    );
    await probe.goto('http://localhost:9876/webauthn-proof');
    const cdp = await context.newCDPSession(probe);
    await cdp.send('WebAuthn.enable');
    const { authenticatorId } = await cdp.send('WebAuthn.addVirtualAuthenticator', {
      options: {
        protocol: 'ctap2',
        transport: 'internal',
        hasResidentKey: true,
        hasUserVerification: true,
        isUserVerified: true,
        automaticPresenceSimulation: true,
      },
    });
    const results = [];
    for (const residentKey of ['discouraged', 'preferred']) {
      await cdp.send('WebAuthn.clearCredentials', { authenticatorId });
      const extensions = await probe.evaluate(async (residentKey) => {
        const credential = await navigator.credentials.create({
          publicKey: {
            challenge: crypto.getRandomValues(new Uint8Array(32)),
            rp: { id: 'localhost', name: 'Identity probe' },
            user: {
              id: crypto.getRandomValues(new Uint8Array(16)),
              name: residentKey,
              displayName: residentKey,
            },
            pubKeyCredParams: [{ type: 'public-key', alg: -7 }],
            authenticatorSelection: {
              residentKey,
              requireResidentKey: false,
              userVerification: 'required',
            },
            extensions: { credProps: true },
            timeout: 10000,
          },
        });
        return credential.getClientExtensionResults();
      }, residentKey);
      const { credentials } = await cdp.send('WebAuthn.getCredentials', { authenticatorId });
      const resident = credentials.length === 1 && credentials[0].isResidentCredential;
      if (resident !== (residentKey === 'preferred')) {
        throw new Error(`Unexpected residency for ${residentKey}`);
      }
      results.push({ residentKey, resident, extensions });
    }
    // The preferred credential must support a usernameless assertion: no
    // selected ID or userHandle is supplied by the test to credentials.get.
    const assertion = await probe.evaluate(async () => {
      const credential = await navigator.credentials.get({
        publicKey: {
          challenge: crypto.getRandomValues(new Uint8Array(32)),
          rpId: 'localhost',
          allowCredentials: [],
          userVerification: 'required',
          timeout: 10000,
        },
      });
      return { userHandlePresent: credential.response.userHandle !== null };
    });
    if (!assertion.userHandlePresent) throw new Error('Missing discoverable userHandle');
    return { browser: context.browser().version(), results, assertion };
  } finally {
    await context.close();
  }
}
