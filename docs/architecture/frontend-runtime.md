# Runtime Frontend

`identity-web` et `drive-web` utilisent le meme modele runtime:

- TanStack Router porte les routes client.
- TanStack Query porte l'etat serveur, le cache, les retries et l'invalidation.
- Effect porte les workflows multi-etapes comme login, OAuth callback, MFA et logout.
- `@nvbes/http-client` est la frontiere HTTP bas niveau et valide les DTOs au runtime.

Les composants React doivent rendre l'UI, lire l'etat query et declencher des mutations. Ils ne doivent pas appeler `fetch` directement, construire des query keys ad hoc ou porter de l'orchestration multi-requetes.

Suffixes a utiliser pour le nouveau code frontend:

- `*.api.ts`: typed HTTP calls with DTO validation.
- `*.workflow.ts`: Effect orchestration around API calls and browser side effects.
- `*.queries.ts`: query key factories, query options, and mutation functions.
- `*.router.tsx`: app-level route tree.

Garder Effect hors des composants leaf sauf execution via `runClientEffect`, `effectQueryFn` ou `effectMutationFn` depuis `@nvbes/web-runtime`.
