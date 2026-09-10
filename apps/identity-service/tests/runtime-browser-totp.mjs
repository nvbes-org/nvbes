// Real HTTPS services and SDK. WebCrypto emulates only the synthetic authenticator.
export async function verifyBrowserTotp(
  browser,
  clientOrigin,
  manageFactors = false,
  policy = 'primary',
) {
  if (!['primary', 'recent_mfa', 'recent_webauthn'].includes(policy))
    throw new Error('Invalid test policy');
  const context = await browser.newContext({ ignoreHTTPSErrors: true });
  try {
    const page = await context.newPage();
    if (manageFactors || policy === 'recent_webauthn') {
      const cdp = await context.newCDPSession(page);
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
    }
    await page.goto(clientOrigin);
    const config = await page.evaluate(async () => (await fetch('/__fixture/config')).json());
    if (policy === 'recent_mfa') config.clientId = config.mfaClientId;
    if (policy === 'recent_webauthn') config.clientId = config.webauthnClientId;
    const authorizationUrl = await page.evaluate(async (config) => {
      const { createAuthorizationRequest } =
        await import('/libs/ts/identity-sdk-web/src/oauth.authorization-request.ts');
      const { BrowserSessionStorage } = await import('/libs/ts/identity-sdk-web/src/storage.ts');
      return (
        await createAuthorizationRequest(
          {
            baseUrl: config.origins.identity,
            clientId: config.clientId,
            redirectUri: config.redirectUri,
            resource: config.origins.account,
            storage: new BrowserSessionStorage(sessionStorage),
          },
          { scope: 'openid account:read' },
        )
      ).authorizationUrl;
    }, config);
    const response = await page.goto(authorizationUrl);
    if (response.status() !== 200) throw new Error('Authorization failed');
    const interaction = await response.json();
    await page.goto(`${config.origins.identity}/__fixture/hosted`);
    const authorized = await page.evaluate(
      async ({ config, interaction, manageFactors, policy }) => {
        const { parseHostedInteraction, loginHostedPassword, completeHostedConsent } =
          await import('/libs/ts/identity-sdk-web/src/hosted.client.ts');
        const { NvbesIdentityWeb } =
          await import('/libs/ts/identity-sdk-web/src/identity-web.client.ts');
        const { HostedIdentityError } =
          await import('/libs/ts/identity-sdk-web/src/hosted.transport.ts');
        const client = new NvbesIdentityWeb({
          baseUrl: config.origins.identity,
          clientId: config.clientId,
          redirectUri: config.redirectUri,
          resource: config.origins.account,
        });
        const transport = { baseUrl: config.origins.identity };
        const initial = parseHostedInteraction(interaction);
        const beforeLogin = await client.getAuthenticationStatus(initial);
        if (
          beforeLogin.minimumAuthentication !== policy ||
          !beforeLogin.needsLogin ||
          beforeLogin.needsStepUp
        )
          throw new Error('Anonymous authentication requirement mismatch');
        const state = await loginHostedPassword(transport, initial, {
          email: config.email,
          password: config.password,
        });
        const expectPolicyDenial = async () => {
          try {
            await completeHostedConsent(transport, state, 'approve');
          } catch (error) {
            if (error instanceof HostedIdentityError && error.status === 400) return;
            throw error;
          }
          throw new Error('Consent bypassed the registered authentication policy');
        };
        const afterPassword = await client.getAuthenticationStatus(state);
        if (afterPassword.needsLogin || afterPassword.needsStepUp !== (policy !== 'primary'))
          throw new Error('Password authentication requirement mismatch');
        if (policy !== 'primary') await expectPolicyDenial();
        const enrollment = await client.startTotpEnrollment(state.sessionCsrfToken);
        if (Date.parse(enrollment.expiresAt) <= Date.now()) throw new Error('Enrollment expired');
        const bytes = [];
        let accumulator = 0;
        let bits = 0;
        for (const character of enrollment.secretBase32) {
          accumulator = (accumulator << 5) | 'ABCDEFGHIJKLMNOPQRSTUVWXYZ234567'.indexOf(character);
          bits += 5;
          if (bits >= 8) {
            bits -= 8;
            bytes.push((accumulator >>> bits) & 255);
          }
        }
        const key = await crypto.subtle.importKey(
          'raw',
          new Uint8Array(bytes),
          { name: 'HMAC', hash: 'SHA-1' },
          false,
          ['sign'],
        );
        const totp = async (counter) => {
          const moving = new ArrayBuffer(8);
          new DataView(moving).setBigUint64(0, BigInt(counter));
          const digest = new Uint8Array(await crypto.subtle.sign('HMAC', key, moving));
          const offset = digest[digest.length - 1] & 15;
          const binary = new DataView(digest.buffer).getUint32(offset) & 0x7fffffff;
          return (binary % 1_000_000).toString().padStart(6, '0');
        };
        const first = await totp(Math.floor(Date.now() / 30_000));
        const expiry = await client.confirmTotpEnrollment(
          state.sessionCsrfToken,
          enrollment.factorId,
          first,
        );
        if (Date.parse(expiry) <= Date.now()) throw new Error('Confirmation did not grant step-up');
        const afterTotp = await client.getAuthenticationStatus(state);
        if (afterTotp.needsStepUp !== (policy === 'recent_webauthn'))
          throw new Error('TOTP policy mismatch');
        if (policy === 'recent_webauthn') {
          await expectPolicyDenial();
          await client.registerPasskey(state.sessionCsrfToken, 'Policy verification key');
          await client.stepUpPasskey(state.sessionCsrfToken);
        }
        const expectReplay = async (code) => {
          try {
            await client.stepUpTotp(state.sessionCsrfToken, code);
          } catch (error) {
            if (error instanceof HostedIdentityError && error.status === 400) return;
            throw error;
          }
          throw new Error('Consumed TOTP accepted twice');
        };
        if (manageFactors) {
          const factors = await client.listTotpFactors(state.sessionCsrfToken);
          if (factors.length !== 1 || factors[0].id !== enrollment.factorId)
            throw new Error('Owned factor metadata missing');
          let refused = false;
          try {
            await client.revokeTotpFactor(state.sessionCsrfToken, enrollment.factorId);
          } catch (error) {
            refused = error instanceof HostedIdentityError && error.status === 409;
          }
          if (!refused) throw new Error('Last strong factor must be preserved');
          await client.registerPasskey(state.sessionCsrfToken, 'Backup for TOTP removal');
        } else {
          await expectReplay(first);
          // Exercise the documented +1 tolerance without a wall-clock sleep or server clock change.
          const next = await totp(Math.floor(Date.now() / 30_000) + 1);
          const nextExpiry = await client.stepUpTotp(state.sessionCsrfToken, next);
          if (Date.parse(nextExpiry) <= Date.now()) throw new Error('Fresh step-up failed');
          await expectReplay(next);
        }
        // A later TOTP step-up can replace the session's current strong-method evidence.
        if (policy === 'recent_webauthn') await client.stepUpPasskey(state.sessionCsrfToken);
        const finalStatus = await client.getAuthenticationStatus(state);
        if (
          finalStatus.needsLogin ||
          finalStatus.needsStepUp ||
          finalStatus.minimumAuthentication !== policy
        )
          throw new Error('Authentication requirement was not satisfied');
        if (policy !== 'primary' && Date.parse(finalStatus.proofExpiresAt) <= Date.now())
          throw new Error('Strong proof expiry missing');
        return {
          redirect: await completeHostedConsent(transport, state, 'approve'),
          csrf: state.sessionCsrfToken,
          factorId: enrollment.factorId,
        };
      },
      { config, interaction, manageFactors, policy },
    );
    await page.goto(authorized.redirect);
    const result = await page.evaluate(async (config) => {
      const { exchangeAuthorizationCode } =
        await import('/libs/ts/identity-sdk-web/src/oauth.authorization-code.ts');
      const { BrowserSessionStorage } = await import('/libs/ts/identity-sdk-web/src/storage.ts');
      const { dpopFetch } = await import('/libs/ts/identity-sdk-web/src/dpop.ts');
      const params = new URL(location.href).searchParams;
      const tokens = await exchangeAuthorizationCode(
        {
          baseUrl: config.origins.identity,
          clientId: config.clientId,
          redirectUri: config.redirectUri,
          storage: new BrowserSessionStorage(sessionStorage),
        },
        { code: params.get('code'), state: params.get('state') },
      );
      history.replaceState(null, '', '/callback');
      const response = await dpopFetch(`${config.origins.account}/api/v1/profile`, {
        key: tokens.dpopKey,
        accessToken: tokens.accessToken,
      });
      window.fixtureTotpAccess = {
        tokens,
        dpopFetch,
        endpoint: `${config.origins.account}/api/v1/profile`,
      };
      return { status: response.status, subject: tokens.identity.subject };
    }, config);
    if (result.status !== 200 || result.subject !== config.subject)
      throw new Error('Account identity mismatch');
    if (manageFactors) {
      const management = await context.newPage();
      await management.goto(`${config.origins.identity}/__fixture/hosted`);
      await management.evaluate(
        async ({ config, authorized }) => {
          const { NvbesIdentityWeb } =
            await import('/libs/ts/identity-sdk-web/src/identity-web.client.ts');
          const { HostedIdentityError } =
            await import('/libs/ts/identity-sdk-web/src/hosted.transport.ts');
          const client = new NvbesIdentityWeb({
            baseUrl: config.origins.identity,
            clientId: config.clientId,
            redirectUri: config.redirectUri,
            resource: config.origins.account,
          });
          await client.revokeTotpFactor(authorized.csrf, authorized.factorId);
          try {
            await client.listTotpFactors(authorized.csrf);
          } catch (error) {
            if (error instanceof HostedIdentityError && error.status === 400) return;
            throw error;
          }
          throw new Error('TOTP revocation must end the requester session');
        },
        { config, authorized },
      );
      const status = await page.evaluate(async () => {
        const { tokens, dpopFetch, endpoint } = window.fixtureTotpAccess;
        return (await dpopFetch(endpoint, { key: tokens.dpopKey, accessToken: tokens.accessToken }))
          .status;
      });
      if (status !== 401)
        throw new Error('TOTP revocation must invalidate existing Account access');
    }
    return {
      browser: browser.version(),
      enrollment: true,
      confirmation: true,
      stepUp: !manageFactors,
      confirmationCodeReplayRefused: !manageFactors,
      stepUpCodeReplayRefused: !manageFactors,
      factorManagementAndApiRevocation: manageFactors,
      oidcAndAccount: true,
      syntheticAuthenticator: true,
      authenticationPolicy: policy,
      passwordOnlyConsentRefused: policy !== 'primary',
      totpConsentRefusedUntilWebauthn: policy === 'recent_webauthn',
    };
  } finally {
    await context.close();
  }
}
