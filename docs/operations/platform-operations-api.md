# Platform Operations — traitement manuel V1

Le runtime existant expose une API JSON et le client Python
`scripts/platform-operations.py`. Les anciennes cartes et actions simulées ont
été retirées. Aucun nouveau service payant n'est nécessaire.

## Stockage et accès

Créer une base **distincte** sur le PostgreSQL existant : le rôle runtime ne
reçoit aucun droit sur les bases Identity, Account, Billing, Email ou Trust/Risk.
Exécuter la migration avec un rôle propriétaire, puis démarrer avec un rôle
runtime non propriétaire et non superutilisateur :

```sh
export NVBES_PLATFORM_OPERATIONS_DATABASE_URL='<URL de migration Operations>'
cargo run --locked --bin nvbes-platform-operations -- migrate
```

Droits du rôle runtime dans cette seule base (adapter son nom) :

```sql
GRANT CONNECT ON DATABASE platform_operations TO platform_operations_runtime;
GRANT USAGE ON SCHEMA public TO platform_operations_runtime;
GRANT SELECT, INSERT, UPDATE ON operations_cases TO platform_operations_runtime;
GRANT SELECT, INSERT ON operations_costs, operations_audit TO platform_operations_runtime;
GRANT USAGE, SELECT ON SEQUENCE operations_audit_sequence_seq TO platform_operations_runtime;
```

Ne pas attribuer de droit CREATE, DELETE, TRUNCATE, de propriété de table ni
d'accès au rôle de migration. Les triggers refusent en plus la modification,
suppression ou troncature des audits et coûts. Une correction de coût est une
nouvelle entrée liée à l'ancienne. Sauvegarder cette base avec la procédure
PostgreSQL existante ; sa restauration n'est jamais déclarée vérifiée par défaut.
Rollback applicatif : arrêter le nouveau runtime en conservant la base et ses
audits. Ne pas réactiver le cockpit simulé ni supprimer les tables pour revenir
à une ancienne image.

Configuration du serveur :

- `NVBES_PLATFORM_OPERATIONS_DATABASE_URL` : URL du rôle runtime Operations ;
- `NVBES_PLATFORM_OPERATIONS_PUBLIC_KEY_PEM` : clé publique RSA de l'émetteur
  opérateur de confiance ; aucune clé privée dans ce service ;
- `NVBES_PLATFORM_OPERATIONS_ISSUER` : valeur exacte du claim `iss` ;
- `NVBES_PLATFORM_OPERATIONS_SERVICES` : tableau JSON optionnel, par exemple
  `[{"service":"account","base_url":"https://account.internal.example"}]` ;
- `NVBES_PLATFORM_OPERATIONS_PORT` : 8084 par défaut.

Le jeton doit être signé RS256, avoir `sub`, `exp`, `nbf`, `iss`,
`aud=platform-operations`, `role=platform_owner`, `amr` contenant une méthode MFA
et `auth_time`. Le rôle doit être attribué par l'émetteur après vérification de
l'opérateur, jamais depuis un champ utilisateur. Une session utilisateur
ordinaire, un bearer statique et l'en-tête `x-nvbes-mfa-step-up` sont insuffisants.
Les futurs rôles doivent recevoir des permissions explicites dans la politique ;
aucun rôle spécialisé n'est activé implicitement.

L'émetteur doit délivrer ce contrat opérateur avant activation en production.
La configuration échoue sans clé/émetteur/base ; il n'existe pas de bypass local.
Le script de preuve utilise sa propre clé éphémère et des données synthétiques.

## Client et commandes

```sh
export NVBES_PLATFORM_OPERATIONS_URL='https://operations.example'
export NVBES_PLATFORM_OPERATIONS_ACCESS_TOKEN='<jeton opérateur MFA>'
python3 scripts/platform-operations.py overview
python3 scripts/platform-operations.py command dossier.json
python3 scripts/platform-operations.py cases
python3 scripts/platform-operations.py case '<case_id>'
python3 scripts/platform-operations.py audits --case-id '<case_id>'
python3 scripts/platform-operations.py costs 2026-09-01
```

La pagination utilise `--after` avec le `next_after` retourné. Arrêter sur une
page vide. La vue des coûts borne les détails à 1 000 lignes, mais ses totaux
portent sur **toutes** les écritures effectives du mois.

Exemple `dossier.json` (générer deux UUID pour chaque nouvelle commande) :

```json
{
  "idempotency_key": "49db2f36-1488-473c-883c-c2707e2c657a",
  "correlation_id": "b147613d-89a1-4d46-88d8-3ec5957b694a",
  "reason": "Demande de vérification de livraison reçue",
  "action": {
    "type": "open_case",
    "category": "support",
    "owner": "email",
    "subject_id": "58fa4945-962e-4fda-9400-f09422a47651",
    "source": "ticket-support-42",
    "summary": "Vérifier la livraison transactionnelle",
    "related_case_id": null
  }
}
```

Catégories : `support`, `security`, `abuse`, `billing`, `appeal`. Domaines :
`identity`, `account`, `billing`, `email`, `trust-risk`. Un dossier de recours
exige `related_case_id` et le même sujet que le dossier d'origine.

L'enveloppe de commande reste identique pour les actions suivantes :

| `action.type`        | Champs supplémentaires                                                                                            |
| -------------------- | ----------------------------------------------------------------------------------------------------------------- |
| `transition`         | `case_id`, `expected_version`, `status`, `evidence`                                                               |
| `add_note`           | `case_id`, `expected_version`, `note`, `evidence`                                                                 |
| `record_observation` | `case_id`, `expected_version`, `service`, `observed_at` RFC3339, `api_reference`, `summary`                       |
| `record_cost`        | `month` YYYY-MM-01, `provider`, `category`, `actual_cents`, `forecast_cents`, `evidence`, `replaces` UUID ou null |

Une réponse de succès fournit l'identité **authentifiée**, le reçu, le dossier,
sa nouvelle version et la corrélation. Une réponse perdue se récupère en rejouant
**le même fichier**, y compris la même clé. Une clé réutilisée avec un contenu
différent ou une version obsolète renvoie 409. L'audit et la mutation sont une
transaction unique : un échec de stockage ne produit aucun succès fictif.

## Parcours de bout en bout

1. Ouvrir le dossier ; conserver le reçu et `case_id`.
2. Passer de `open` à `investigating` avec la version courante et une référence.
3. Consulter les APIs opérateur autorisées du domaine propriétaire. Enregistrer
   une `record_observation` par domaine utile, en minimisant les données.
   Le détail du dossier consolide la dernière observation de chaque domaine,
   sa date et son auteur, ainsi que la disponibilité HTTP des services configurés.
   Les sondes HTTP ne prouvent pas un état utilisateur. Une observation est une
   attestation manuelle datée, pas une réplication automatique de la base métier.
4. Ajouter les notes de qualification, décision et notification avec `add_note`.
5. Utiliser `awaiting_user` puis `investigating` pour les échanges. Utiliser
   `action_pending` si une intervention du domaine propriétaire est nécessaire ;
   revenir à `investigating` seulement après vérification du résultat exact.
6. Passer à `resolved` avec la preuve du résultat et de la notification, puis
   `closed` après vérification. Un état en attente ne peut pas être clôturé
   directement. Les preuves sont des références minimisées, jamais des secrets,
   credentials, messages complets ni exports de bases.
7. Un recours rouvre `resolved` ou `closed` vers `appealed`, puis
   `investigating`, ou ouvre un dossier `appeal` lié au précédent.
8. Consulter la chronologie via `audits --case-id` ; les motifs, observations,
   notes et reçus y restent consultables après clôture.

## Limite des actions métier

Cette livraison exécute les commandes **de dossier et de registre**. Elle ne
prétend pas exécuter une suppression Email, un remboursement ou une suspension
de compte. Les anciens appels `/api/v1/actions` renvoient 501 avec
`executed=false` : leurs anciens reçus de succès étaient simulés.

Les interventions métier restent dans les outils opérateur autorisés du domaine,
avec leur propre audit ; consigner leur reçu exact dans le dossier. Si leur
contrat ne garantit pas identité, idempotence, motif, audit et step-up pour une
action sensible, **ne pas exécuter**, garder le dossier en attente. Aucune
usurpation de session, suppression irréversible ou délégation de bearer
utilisateur n'est exposée. L'activation d'adaptateurs d'exécution métier exige
leurs garanties propres et ne peut pas être remplacée par un bouton simulé.

## FinOps et validation

Chaque écriture est un montant EUR TTC et une prévision du mois entier, pas un
montant à extrapoler. Les corrections référencent `replaces` et conservent mois,
fournisseur et catégorie. Les seuils sont 20/25/28/30 EUR. Déclarer un dépassement
reste possible : le registre ne doit jamais masquer une facture réelle.
Un registre vide reste `unknown` ; un total sous plafond ne prouve pas que tous
les fournisseurs sont saisis. Vérifier manuellement la couverture.

Coût fixe ajouté : 0 EUR de service fournisseur. Réutilisation du runtime
scale-to-zero et de l'instance PostgreSQL existants, pool limité à deux connexions,
requêtes et pages bornées, aucun polling automatique. Attribuer stockage,
sauvegardes et durée des sessions au contrat global existant ; aucune hausse de
capacité ni nouveau fournisseur n'est autorisé par ce changement. Vérifier
`pnpm check:finops` et les factures avant tout déploiement.

```sh
pnpm nx run platform-operations-service:test
DATABASE_URL='<PostgreSQL de test avec CREATEDB>' pnpm nx run platform-operations-service:test:database
pnpm nx run rust-workspace:check
NVBES_PLATFORM_OPERATIONS_DATABASE_URL='<base de preuve jetable>' \
  bash tools/deployment/prove-platform-operations-runtime.sh
```

La preuve démarre un véritable serveur, vérifie authentification signée,
persistance après redémarrage, conflits, observations, clôture/recours, audits et
coûts. Elle n'envoie aucun message et ne modifie aucun service métier.
