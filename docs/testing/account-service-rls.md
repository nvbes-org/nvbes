# Account Service isolation (V1)

> **Statut : runtime lean actif.** Ce document remplace le contrat RLS
> Enterprise historique. Aucune preuve V1 ne s'appuie sur
> `identity.database.rls.reference.sql` ni sur un modèle multi-tenant forcé.

## Modèle V1

Account isole les données par principal et membership d'équipe :

- tables profil / préférences / consents / exports / closures clés sur
  `principal_id` ;
- memberships `account_team_memberships` avec rôles `owner` | `member` ;
- un seul owner par équipe (index unique partiel) ;
- requêtes HTTP bornées au principal authentifié ou à l'appartenance équipe ;
- un owner seul peut retirer un membre ; un owner ne quitte que s'il est le
  dernier membre (équipe alors `closed`).

Il n'y a pas de `FORCE ROW LEVEL SECURITY` ni de contexte tenant transactionnel
dans le runtime V1. L'isolation repose sur les prédicats SQL des handlers et sur
les contraintes de schéma.

## Preuves

- `account.database.tests.rs` : lifecycle synthétique (join, leave, export,
  closure, outbox) ;
- handlers équipes : `NotFound` si hors membership, `Forbidden` si non-owner
  retire un membre, `Conflict` si owner quitte avec d'autres membres.

## Hors périmètre V1

- RLS Enterprise, SAML, workspaces multi-tenant, bootstrap nil-UUID ;
- tout harness RLS référencé dans l'archive `account-service-next`.
