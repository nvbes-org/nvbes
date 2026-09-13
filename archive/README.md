# Archives hors socle V1

Applications parked while the platform is rebuilt service by service.

Archived sources stay versioned under `archive/apps/`, but are excluded from the
active Cargo, pnpm, and Nx workspaces. Restore an application by moving it back
to `apps/` and explicitly reconnecting its workspace configuration.

The email service remains active in `apps/email-worker`.

Cloud/Drive et Enterprise sont hors de la direction V1 : leurs applications
restent dans `archive/apps/`, leurs crates produit dans `archive/libs/rust/products/`
et les anciens clients Enterprise/fédération dans `archive/libs/ts/identity-client/`.
Ces derniers sont des sources historiques, pas un package publiable autonome.
Leurs anciens imports relatifs sont conservés pour référence ; une restauration
exige une conception et une reconnexion explicites, pas une compilation automatique.
Le package actif `@nvbes/identity-client` ne publie que la surface Account.

Cette exclusion découle de la direction produit, pas des scores de couverture.
Les tests et seuils du code Account et des primitives partagées restent obligatoires.

Les scripts historiques de migration Cloud, migration staging multi-produit,
génération OpenAPI et parcours E2E dépendant de Cloud sont dans `archive/scripts/`.
Leurs anciennes commandes racine ne sont plus exposées. Les runtimes V1 ont leurs
commandes `migrate` propres ; `pnpm dev` les orchestre en local, et
`pnpm db:migrate:billing` conserve la migration Billing explicite.
La génération OpenAPI historique n'est pas une capacité des nouveaux services.
Le fuzzing des uploads Cloud est conservé dans `archive/fuzz/`, hors campagne V1.

L'ancien adaptateur Redis et ses primitives de cache, verrouillage, limitation et
files de travail sont conservés dans `archive/libs/rust/redis/`. Ils ne font plus
partie du workspace Cargo actif : les primitives de sécurité encore requises ont
une source de vérité PostgreSQL, tandis que les anciennes files Redis restent une
référence historique non déployable.
