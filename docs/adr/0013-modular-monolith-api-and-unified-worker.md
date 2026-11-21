# 0013. Monolithe Modulaire API et Worker Unifié

Date: 2026-11-21  
Statut: Accepté  
Décideurs: Équipe nvbes  

## Contexte

Le socle initial nvbes V1 répartissait la logique métier sur plusieurs microservices HTTP distincts (`identity-service`, `account-service`, `billing-service`) et plusieurs workers (`email-worker`, `billing-worker`).

Cette dispersion posait des contraintes fortes :
- **Budget FinOps (< 30 EUR TTC/mois)** : multiplier les conteneurs et les pools de connexions PostgreSQL (`PgPool`) entraînait un surcoût d'infrastructure et d'empreinte mémoire inutile pour une cible V1 solo.
- **Complexité réseau & Latence** : l'authentification et l'autorisation inter-services nécessitaient des requêtes HTTP d'introspection en boucle locale.
- **Charge opérationnelle** : gestion de multiples pipelines Docker et gates de livraison pour un opérateur unique.

## Décision

Nous adoptons une architecture en **Monolithe Modulaire Serverless** :
1. **Un binaire API HTTP serverless pur (`apps/api`)** :
   - Zéro sous-commande CLI dans le binaire API. Il démarre instantanément en lisant les variables d'environnement et écoute sur `PORT` (Scaleway Serverless Containers / Cloud Run).
   - Conçu pour le **scale-to-zero** (`min_scale = 0`), stateless avec cold start < 100ms et graceful shutdown sur `SIGTERM`.
   - Pool de connexions SQLx adapté au serverless (`min_connections = 0`, `idle_timeout` court).
   - Les migrations sont externalisées hors du binaire API (exécutées par la CI/CD ou un Serverless Job dédié).
2. **Un binaire Worker / Serverless Jobs (`apps/worker`)** :
   - Centralise toutes les tâches d'arrière-plan (Transactional Outbox, envoi d'emails Scaleway TEM, réconciliation Stripe, purges RGPD).
   - Déployé en tant que conteneur worker ou Serverless Job planifié.
3. **Crates de domaine étanches dans `libs/rust/`** (`libs/rust/identity`, `libs/rust/account`, `libs/rust/billing`, `libs/rust/consent`). Le binaire `apps/api` sert uniquement de composition root.
4. **Base PostgreSQL unique avec schémas séparés** (`identity.*`, `account.*`, `billing.*`, `consent.*`, `outbox.*`) et zéro clé étrangère directe inter-schémas.
5. **Communication in-process par Traits Rust (Ports & Adapters)** : `IdentityPort::introspect` élimine les boucles réseau pour la vérification des jetons et de l'autorité. Les effets de bord utilisent la Transactional Outbox.
6. **Routage rétro-compatible** : conservation des chemins OAuth/OIDC (`/.well-known/*`, `/oauth/*`) et REST (`/api/v1/*`) sans rupture pour les clients frontend (`identity-web`, `account-web`).
7. **Contrat OpenAPI consolidé** : génération via `utoipa` synchronisée vers les types TypeScript.

## Conséquences

### Positives
- **FinOps strict respecté** : réduction drastique de l'empreinte mémoire et CPU, 1 seul conteneur pour l'API et 1 pour le worker.
- **Latence zéro inter-domaines** : pas de round-trips HTTP/gRPC internes pour l'introspection de tokens.
- **Simplicité opérationnelle** : 2 conteneurs à déployer et surveiller au lieu de 6+.
- **Isolation préservée** : modularité stricte au niveau du compilateur Rust via les crates séparées.

### Négatives / Risques
- Nécessite une migration méthodique par étapes pour extraire les crates de domaine sans régression sur les suites de tests existantes.
