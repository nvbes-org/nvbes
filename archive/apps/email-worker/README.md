# nvbes Email Worker

> **Note d'Architecture (ADR 0013)** : Le dispatching des emails transactionnels Scaleway TEM et la gestion de la Transactional Outbox convergent vers le worker centralisé serverless [apps/worker](../worker/README.md) (`nvbes-worker`).

Worker d'envoi et d'orchestration des emails transactionnels nvbes.

## Responsabilités

- Consommation de la file d'attente des emails transactionnels (SQS / file locale).
- Rendu des templates HTML via le composant React Email (`libs/ts/email-ui`).
- Délégation de l'envoi aux adaptateurs de messagerie (Scaleway Email, SMTP local).
- Gestion des retries, du backoff exponentiel et des dead-letter queues.

## Commandes

```bash
# Lancement local avec rechargement à chaud
pnpm dev:email-worker

# Tests de base de données du worker
pnpm test:email-worker:database
```
