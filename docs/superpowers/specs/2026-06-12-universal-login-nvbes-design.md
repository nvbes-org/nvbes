# Universal Login nvbes Design

## Goal

Build nvbes Identity Universal Login as the hosted authentication entrypoint for every nvbes product.

The first complete v1 must let a product start an OAuth authorization-code + PKCE flow, send the user to the hosted Identity login, let the user choose an account when relevant, approve consent when required, and return to the product with either a valid authorization code or a redirect-safe OAuth error.

## Scope

In scope:

- hosted `/oauth/authorize` entrypoint for browser-based products;
- login required, account chooser, consent required, success, and OAuth error states;
- client and tenant branding data exposed to the hosted login UI;
- reuse of the existing challenge login flow: identifier, password, MFA;
- PAR-compatible backend flow, because the current authorization endpoint already requires pushed authorization requests;
- authorization-code issue after login and consent;
- targeted backend and frontend tests.

Out of scope:

- SAML and SCIM;
- social login;
- advanced custom CSS branding;
- organization discovery beyond existing tenant/workspace context;
- admin consent workflows;
- passkeys as a required first factor;
- a new token format.

## Architecture

Universal Login stays split across `identity-api` and `identity-web`.

`identity-api` remains the security boundary. It validates the OAuth request, client, redirect URI, PKCE, requested scopes, session, workspace context, and consent status. It produces explicit flow decisions rather than letting the frontend infer security behavior.

`identity-web` remains the hosted product UI. It reads the Universal Login state, reuses the existing login challenge components, shows account chooser and consent screens, applies branding from the backend response, and redirects only through backend-approved decisions.

## Flow

1. A product redirects the browser to `/oauth/authorize` with OAuth parameters.
2. If the request is not already a valid PAR request, Identity creates or resolves a hosted authorization state and redirects the browser to `/login` with a state reference.
3. The hosted login page loads the authorization state from Identity.
4. If no valid session exists, the existing challenge login flow runs.
5. If multiple local accounts or sessions are available, the user selects one.
6. Identity evaluates whether consent is required for the client and scopes.
7. If consent is required, `identity-web` displays the client name, scopes, workspace/tenant context, and approve/cancel actions.
8. On approval, Identity creates the authorization code and returns a backend-approved redirect target.
9. `identity-web` navigates to the redirect target.
10. On cancel or redirect-safe OAuth errors, Identity returns an OAuth error redirect target with `error`, `error_description`, and `state` when available.
11. On non-redirect-safe errors, `identity-web` shows a hosted error page.

## Backend Contracts

The backend should expose a small hosted-login API under `/api/v1/oauth/hosted-login` or the closest existing OAuth module route pattern:

- `POST /api/v1/oauth/hosted-login/start`
  - accepts the browser authorization parameters or a PAR `request_uri`;
  - validates client, redirect URI, response type, PKCE, scopes, and request shape;
  - stores an opaque hosted authorization state in Redis;
  - returns a hosted login URL or an immediate redirect decision.

- `GET /api/v1/oauth/hosted-login/{stateId}`
  - returns client display data, requested scopes, tenant/workspace context, and the current flow step;
  - never returns secrets, raw PAR payloads, authorization codes, or object keys.

- `POST /api/v1/oauth/hosted-login/{stateId}/authorize`
  - requires a valid user session or session token;
  - evaluates workspace context and consent;
  - returns either `redirect`, `consent_required`, `account_required`, or `error_page`.

- `POST /api/v1/oauth/hosted-login/{stateId}/consent`
  - records approval or denial;
  - on approval, issues the authorization code;
  - on denial, returns an OAuth `access_denied` redirect when redirect-safe.

The exact route names may be adjusted to match existing module conventions, but the contract must preserve these boundaries.

## Frontend Behavior

`identity-web` should treat `/login` as the Universal Login shell when an OAuth state or OAuth request is present.

The UI has five explicit states:

- `loading`: resolve hosted login state;
- `login`: run the existing challenge flow;
- `account_chooser`: select an existing account/session;
- `consent`: approve or deny client access;
- `oauth_error`: show non-redirect-safe OAuth errors.

The existing account pages remain separate. A normal visit to `/login` without OAuth context should continue to behave as the direct Identity login.

## Data And Security

- Hosted authorization state is opaque, short-lived, and stored server-side.
- Redirects are only built from previously validated redirect URIs.
- Public clients require PKCE with `S256`.
- Authorization codes remain short-lived and one-time-use.
- Consent and denial are audited.
- Errors that can safely redirect follow OAuth error parameters.
- Errors that cannot safely redirect stay on the hosted error page.
- The frontend never constructs a redirect URI from unvalidated user input.

## Testing

Backend tests should cover:

- invalid client;
- invalid redirect URI;
- public client without PKCE rejected;
- unauthenticated OAuth request produces login-required hosted state;
- authenticated approval issues an authorization code;
- consent denial returns `access_denied`;
- non-redirect-safe errors do not redirect.

Frontend tests should cover:

- OAuth request is preserved through login;
- login completion resumes hosted authorization;
- consent approval follows backend redirect;
- consent cancellation follows backend error redirect;
- hosted OAuth error renders without exposing raw internals.

## Rollout

The feature should be built behind the existing local/staging Identity deployment path. It should not change Drive-specific behavior except enabling Drive or future products to use the hosted OAuth login URL.

Validation after implementation:

- targeted Rust tests for OAuth hosted login;
- targeted React tests for login/consent state;
- `cargo check --workspace`;
- targeted identity-web typecheck/test command;
- update API docs if public route contracts change.
