# `@nvbes/account-client`

Typed browser client for the Account OAuth resource server.

The caller owns the OAuth lifecycle and injects the current access token. The
client resolves that token before every request, always sends it as
`Authorization: Bearer …`, and always uses `credentials: "omit"`.

```ts
import { createAccountClient } from '@nvbes/account-client';

const account = createAccountClient({
  baseUrl: 'https://account.nvbes.fr',
  getAccessToken: () => accessTokenStore.current(),
});

const profile = await account.getProfile();
```

The public surface is deliberately restricted to Account capabilities:

- profile and avatar (`/api/v1/profile*`);
- display and notification preferences (`/api/v1/preferences` and
  `/api/v1/notifications`);
- legal consents (`/api/v1/consents`);
- GPC status and personal-data export (`/api/v1/privacy/*`);
- account closure (`/api/v1/closure`).

Login, email identities, MFA, sessions, devices, and OAuth client grants belong
to Identity and are intentionally absent. The client contains no aliases for
the former Identity-owned HTTP surfaces.
