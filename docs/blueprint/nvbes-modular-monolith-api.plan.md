# Plan d'Architecture : Monolithe Modulaire API (`apps/api`) & Worker Centralisé (`apps/worker`)

> **Source de vérité de conception** : Validé lors de la session `/grill-me` du 21 Novembre 2026.  
> **ADR Associé** : [0013-modular-monolith-api-and-unified-worker.md](../adr/0013-modular-monolith-api-and-unified-worker.md)  
> **Référence Produit** : [Direction produit nvbes V1](../product/nvbes-product-strategy.md)  

---

## 1. Vue d'Ensemble & Périmètre V1

Le backend nvbes V1 est restructuré autour de deux composants d'exécution serverless :
1. **`apps/api` (Serverless HTTP Engine)** :
   - Déployé sur **Scaleway Serverless Containers** avec `min_scale = 0` (scale-to-zero) pour un coût nul en période d'inactivité.
   - **Zéro sous-commande CLI** dans le binaire API : pur serveur HTTP Axum 0.8 démarrant directement sur `$PORT` avec lecture exclusive des variables d'environnement.
   - Démarrage instantané (cold start < 100ms), 100% stateless, avec gestion de `SIGTERM` pour graceful shutdown.
   - Pool PostgreSQL SQLx calibré pour le serverless (`min_connections = 0`, connexions relâchées rapidement).
   - Regroupe les quatre domaines fondamentaux :
     - **Identity** (OAuth 2.1, OIDC Discovery, PKCE, PAR, DPoP, WebAuthn/Passkeys, TOTP, Recovery)
     - **Account** (Profils utilisateurs, préférences, gestion des équipes)
     - **Billing** (Abonnements, Stripe Checkout & Customer Portal, gestion des quotas)
     - **Consent** (Consentements légaux CGU/Privacy, politique cookies avec support Sec-GPC, réconciliation anonyme $\to$ authentifié, audit RGPD)
2. **`apps/worker` / Serverless Jobs** :
   - Moteur d'arrière-plan centralisé (tâche conteneur ou Scaleway Serverless Jobs) :
     - Dépilage de la table Transactional Outbox
     - Dispatch des emails transactionnels via Scaleway TEM
     - Réconciliation et synchronisation des événements Stripe
     - Tâches planifiées de rétention et purge RGPD (délai de 30 jours)

*Note : La composante Platform Operations (administration opérateur) est volontairement différée pour cette phase.*

---

## 2. Modèle de Données & Schémas PostgreSQL

Une seule instance managée PostgreSQL 17 avec un pool de connexions SQLx unique (`PgPool`), divisée en schémas logiques étanches :

```sql
CREATE SCHEMA IF NOT EXISTS identity;
CREATE SCHEMA IF NOT EXISTS account;
CREATE SCHEMA IF NOT EXISTS billing;
CREATE SCHEMA IF NOT EXISTS consent;
CREATE SCHEMA IF NOT EXISTS outbox;
```

### Règles d'or de modélisation
1. **Zéro Foreign Key transverse** : Aucun `REFERENCES identity.users(id)` dans `account`, `billing` ou `consent`. Les identifiants sont des types `UUID` sans contrainte SQL inter-schéma.
2. **Migrations SQLx co-localisées** :
   - `libs/rust/identity/migrations/`
   - `libs/rust/account/migrations/`
   - `libs/rust/billing/migrations/`
   - `libs/rust/consent/migrations/`
   L'orchestration au démarrage applique séquentiellement les migrations de chaque crate via `sqlx::migrate!`.

---

## 3. Communication Inter-Domaines (In-Process)

Pour garantir l'indépendance des modules tout en profitant des performances de la mémoire partagée :

```rust
// Trait d'introspection et de sécurité
#[async_trait::async_trait]
pub trait IdentityPort: Send + Sync {
    async fn introspect(
        &self,
        token: &str,
        dpop_proof: Option<&str>,
        expected_resource: Option<&str>,
    ) -> Result<Principal, IdentityError>;
}

// Trait de validation des consentements
#[async_trait::async_trait]
pub trait ConsentPort: Send + Sync {
    async fn has_accepted_latest_tos(&self, user_id: Uuid) -> Result<bool, ConsentError>;
    async fn reconcile_anonymous_consent(
        &self,
        anonymous_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), ConsentError>;
}

// Trait d'autorisation de compte
#[async_trait::async_trait]
pub trait AccountPort: Send + Sync {
    async fn authorize_team_member(
        &self,
        user_id: Uuid,
        team_id: Uuid,
        required_role: TeamRole,
    ) -> Result<(), AccountError>;
}
```

Les effets de bord (création de compte suite à enregistrement, notifications par email, initialisation de client Stripe) s'effectuent par écriture atomique dans la table `outbox.messages` lors de la transaction locale, puis sont asynchronisés vers `apps/worker`.

---

## 4. Spécification Détaillée du Domaine `Consent`

La crate `libs/rust/consent` gère :
1. **Accords légaux** :
   - Types : `terms_of_service`, `privacy_policy`.
   - Suivi par version de document (`document_version`).
   - Traçabilité : horodatage, hachage d'IP, User-Agent.
2. **Préférences cookies & traceurs** :
   - Catégories : `functional`, `analytics`, `marketing`.
   - Prise en charge automatique de l'en-tête `Sec-GPC: 1` (Global Privacy Control) pour désactiver les traceurs non nécessaires.
3. **Réconciliation anonyme $\to$ authentifié** :
   - Cookie visiteur `nvbes_consent_id` rattaché à `user_id` lors de la connexion.
4. **Audit trail immuable** :
   - Journalisation de tout changement dans `consent.audit_events`.

---

## 5. Routage & OpenAPI

L'API conserve une compatibilité intégrale avec les clients et SDKs existants :
- **Racine OAuth / OIDC** : `/.well-known/openid-configuration`, `/.well-known/jwks.json`, `/oauth/*`
- **Endpoints REST v1** : `/api/v1/profile`, `/api/v1/teams`, `/api/v1/consents`, `/api/v1/billing/*`
- **Probes de plateforme** : `/health`, `/ready`, `/metrics`

Un contrat OpenAPI consolidé est généré avec `utoipa` (`apps/api/openapi.json`) et synchronisé avec les bibliothèques TypeScript via `pnpm codegen`.

---

## 6. Plan d'Implémentation par Étapes

1. **Étape 1** : Création/extraction des crates de domaine dans `libs/rust/` :
   - `libs/rust/consent` (schéma, entités, routes)
   - `libs/rust/identity` (migration depuis `apps/identity-service`)
   - `libs/rust/account` (migration depuis `apps/account-service`)
   - `libs/rust/billing` (ajustement du routeur Axum)
2. **Étape 2** : Assemblage de `apps/api` :
   - Bootstrapping Axum 0.8, injection des ports, middlewares unifiés (DPoP, CORS, SecHeaders).
   - Orchestration des migrations SQLx multi-schémas.
3. **Étape 3** : Centralisation du worker dans `apps/worker` :
   - Fusion de `apps/email-worker` et `apps/billing-worker`.
   - Dispatcher Outbox, Scaleway TEM, Stripe, purge RGPD.
4. **Étape 4** : Validation des contrats OpenAPI et SDKs TypeScript.
5. **Étape 5** : Validation des tests E2E, mise à jour des gates `lefthook.yml` / CI, et archivage des anciens services dans `archive/apps/`.
