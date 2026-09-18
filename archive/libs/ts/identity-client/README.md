# @nvbes/identity-client

Client TypeScript typé pour les capacités Account liées à Identity.

## Fonctionnalités

- Profil, adresses email, sessions, consentements et autorisations OAuth.
- Accès via `AccountIdentityClient`, `createAccountIdentityClient` et `accountClient`.

Les clients Enterprise, fédération et revues d'accès sont archivés sous
`archive/libs/ts/identity-client/` et ne sont plus exportés. Cloud et Enterprise
ne font pas partie de la V1. Les flux navigateur OAuth/WebAuthn/MFA sont portés
par `@nvbes/identity-sdk-web`, pas par ce package.
