---
title: Service Billing & Abonnements
description: Architecture du moteur de facturation, gestion multi-PSP (Stripe), webhooks idempotents et gestion des droits.
---

## 1. Rôle Architectural

Gestionnaire des souscriptions, du calcul d usage et de la synchronisation des états de facturation.

- **Domaine métier** : `billing`
- **Mécanisme d'authentification** : gRPC mTLS interne + Signature HMAC des webhooks Stripe
- **Scopes requis** : `billing:read`, `billing:admin`

---

## 2. Invariants & Règles Métier

Ces règles constituent les garanties fondamentales du service :

- **Règle** : Tous les webhooks PSP (Stripe) doivent être traités avec une idempotence stricte.
- **Règle** : Le routage des paiements est neutre vis-à-vis des fournisseurs (ADR 0004).
- **Règle** : Aucun changement de statut de souscription n est appliqué sans audit append-only.

---

## 3. Flux Principal (Diagramme de Séquence)

```mermaid
sequenceDiagram
    autonumber
    actor Stripe as "Stripe PSP Webhook"
    participant GW as "gateway-cloud"
    participant Billing as "billing-service"
    participant DB as "PostgreSQL (Billing DB)"
    participant Worker as "billing-worker"

    Stripe->>GW: POST /webhooks/stripe (Event payload + Sig)
    GW->>Billing: Transmet le webhook avec verification HMAC
    Billing->>Billing: Verifie la cle d idempotence
    Billing->>DB: Enregistre l evenement brut dans l audit ledger
    Billing->>Worker: Publie la tache asynchrone d ajustement des droits
    Billing-->>Stripe: 200 OK (Recu)
    Worker->>DB: Met a jour les entitlements du compte client
```

---

## 4. Matrice des Erreurs & Stratégies de Résilience

| Type d'Erreur | Code HTTP | Stratégie de Récupération |
| :--- | :--- | :--- |
| `InvalidWebhookSignature` | `400` | Rejeter immédiatement la requête et alerter la sécurité. |
| `DuplicateEventIgnored` | `200` | Renvoyer 200 OK sans retraiter l événement (Idempotence). |
| `PaymentFailed` | `402` | Notifier le client par email et entrer en période de grâce. |
