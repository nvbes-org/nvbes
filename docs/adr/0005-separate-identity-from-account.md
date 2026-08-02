# ADR 0005 - Separate Identity from Account

## Status

Accepted.

This decision supersedes the Account authentication ownership defined by
`docs/adr/2026-07-05-account-cloud-service-taxonomy.md`. The runtime taxonomy
remains valid for the other products.

## Context

`account-service` currently combines two bounded contexts:

- Identity owns principals, login identifiers, credentials, MFA, WebAuthn,
  authentication sessions, risk evaluation, OAuth/OIDC protocol execution,
  token issuance, token introspection and signing keys.
- Account owns the user-facing profile, preferences, contact data, legal
  choices, privacy exports and account lifecycle orchestration.

The code already implements an OAuth 2.1 and OpenID Connect authorization
server, and Account routes already declare Account-specific scopes. However,
first-party browser sessions are still accepted directly by Account handlers,
the Identity issuer is deployed as `account-service`, and at least one resource
server accepts tokens issued for both Account and itself.

This coupling prevents Account from being authenticated and authorized through
the same contracts as the other nvbes products. It also makes credentials,
profile data and product authorization part of the same deployment and data
ownership boundary.

## Decision

Create `identity-service` as the sole nvbes OAuth 2.1 Authorization Server and
OpenID Connect Provider.

Identity owns:

- global principals and login identities;
- passwords, passkeys, WebAuthn, MFA and recovery mechanisms;
- authentication sessions, trusted devices, risk signals and step-up grants;
- OAuth clients at runtime, grants, consents and refresh-token families;
- authorization, token, introspection, revocation, UserInfo, discovery and
  JWKS endpoints;
- access-token, ID-token and security-event signing.

Keep `account-service` as an OAuth Resource Server. Account owns:

- profile and display data that is not an authentication identifier;
- communication preferences and Account UI preferences;
- contact data that is not used to prove identity;
- legal acceptances, privacy requests and export state;
- orchestration of account closure across product services.

Authentication email addresses and their verification state belong to
Identity. Account may consume an event-backed projection when it needs to
display them, but it must not mutate Identity tables.

`account-web` is an OpenID Connect client. It authenticates through
Authorization Code with PKCE and calls `account-service` with an access token
whose audience is exactly `nvbes-account-service`. The Identity SSO cookie is
scoped to Identity endpoints and is never accepted by Account APIs.

Every resource server accepts only its own audience:

| Resource server | Required audience |
| --- | --- |
| `account-service` | `nvbes-account-service` |
| `cloud-service` | `nvbes-cloud-service` |
| `billing-service` | `nvbes-billing-service` |
| `developer-service` | `nvbes-developer-service` |
| `enterprise-service` | `nvbes-enterprise-service` |
| `backoffice-service` | `nvbes-backoffice-service` |

OAuth scopes provide coarse API capabilities. Each service remains responsible
for its domain authorization and must evaluate local RBAC/ABAC or a dedicated
policy decision for resource-level access.

OAuth-facing machine identities use Client Credentials. A service acting on
behalf of a user uses OAuth Token Exchange with a target audience and
constrained scopes. Services must not forward a user token to a different
audience. Private outbox delivery endpoints use a dedicated, narrowly scoped
service credential and are not exposed as OAuth resource APIs.

Identity and Account use separate persistence ownership. Cross-context changes
use versioned events and outbox delivery. Synchronous calls are reserved for
operations requiring an immediate security decision.

## Migration

The cutover is performed through complete contract boundaries:

1. Enforce one exact audience per existing resource server.
2. Introduce Identity-named runtime configuration and deployment contracts.
3. Move OAuth/OIDC, credentials, sessions and MFA into `identity-service`.
4. Move profile, preferences, legal and privacy state into the Account data
   model.
5. Register `account-web` as an OIDC public client and require Account bearer
   tokens on all Account API routes.
6. Remove direct browser-session authentication from `account-service`.
7. Split Identity and Account databases and replace cross-database writes with
   events or explicit service contracts.

A migration step is complete only when its old runtime path is removed. Runtime
aliases and dual ownership are not accepted as a final state.

## Consequences

Identity becomes independently deployable, auditable and hardenable. Account
uses the same OAuth contract as Cloud, Billing, Developer and Enterprise.
Compromised tokens have a smaller blast radius because an Account token cannot
be replayed against Cloud and vice versa.

The split introduces an additional service, separate persistence, event
delivery and more explicit client registration. Account deletion becomes a
distributed workflow and requires idempotency, retries and reconciliation.

The Account UI calls only Account APIs for user-facing account management.
When an operation targets an Identity-owned security primitive, such as MFA or
an authentication session, Account acts as the BFF and invokes Identity through
an explicit audience-bound service contract. Identity Web contains only hosted
authentication and OAuth/OIDC protocol screens; it exposes no account-management
navigation or self-service application shell.

## Validation

The migration is blocked unless:

- each resource server rejects every foreign audience;
- Identity is the only service that issues tokens and owns credentials;
- Account APIs reject Identity browser cookies;
- Account has no direct write access to Identity persistence;
- Identity has no direct write access to Account persistence;
- OAuth clients request explicit resource audiences and minimal scopes;
- Authorization Code uses PKCE with `S256`;
- service-to-service delegation uses Client Credentials or Token Exchange;
- `cargo check --workspace` and the product-boundary checks pass.
