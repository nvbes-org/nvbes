# ADR 0003 - Plateforme Billing Interne Multi-Provider

## Statut

Accepté avec périmètre V1 réduit. La décision provider-neutral et la propriété
Billing restent actives ; la plateforme financière complète, le multi-PSP et
les fonctions fiscales avancées sont futures. Voir la
[direction produit V1](../product/nvbes-product-strategy.md).

## Date

2026-06-21

## Decision

nvbes devient la source de verite billing pour le catalogue produit, les prix versionnes, les entitlements, les usages, les subscriptions canoniques, les invoices, les paiements, le ledger, la reconciliation et les exports finance.

Stripe reste le provider de paiement principal pour la V1. Mollie est modele comme provider secondaire et peut etre active par routage explicite quand les flux canoniques sont stables.

Les providers de paiement executent:

- Customer et checkout provider.
- Portal ou changement de moyen de paiement securise.
- Paiement, mandat, refund provider et dispute provider.
- Webhooks signes et evenements provider retenus pour audit/replay.

Les providers ne decident pas:

- Du plan produit actif.
- Des droits d'acces et quotas.
- De la facture canonique nvbes.
- Du ledger interne.
- Des metriques finance et business.

## Regles d'architecture

- Les nouveaux modeles metier ne portent pas de colonne `stripe_*`; les identifiants externes passent par des mappings provider-neutral.
- Les APIs publiques exposent `provider`, `checkout_id`, `payment_id` et `status`. Les IDs provider bruts restent limites aux endpoints admin explicites.
- Les futurs produits produisent leurs usages et lisent des entitlement
  snapshots projetés ; aucun produit Cloud/Drive n'est actif en V1.
- Billing est le control plane billing canonique; Identity reste le control plane utilisateur, session, workspace membership et autorisation.
- Le ledger interne est append-only. Les refunds, credit notes, write-offs et overrides produisent des ecritures explicites et des audit events.
- Les payloads provider sensibles sont minimises: resume non sensible par defaut, retention brute chiffree ou classee selon politique.

## Limites

nvbes ne devient pas PSP, banque, acquereur, reseau carte ou Merchant of Record. nvbes ne collecte ni PAN ni CVV et garde le scope PCI reduit fourni par Stripe/Mollie.

Le calcul fiscal, l'e-invoicing, la retention des factures et la revenue recognition auditee exigent validation externe avant lancement payant multi-pays.

## Consequences

- Le schema billing canonique vit cote Billing. Tant que le schema physique reste partage en V0, seules les crates `nvbes-billing`, `billing-service` et `billing-worker` peuvent porter les nouvelles responsabilites metier billing.
- Stripe V1 reste supporte via mappings et vues de compatibilite.
- Mollie peut etre reference sans dupliquer le domaine financier.
- Les exports finance et BI futurs doivent provenir du modèle nvbes, pas
  uniquement des pipelines provider.

## Runtime cible V0

- `apps/billing-service`: API serverless-compatible pour endpoints billing internes et futurs resolvers GraphQL.
- `apps/billing-worker`: worker separe pour reconciliation, processing PSP et jobs finance.
- `contracts/protobuf/nvbes/billing/v1/billing.proto`: contrat gRPC interne.
- `contracts/graphql/schema.graphql`: contrat gateway/BFF, gouverne par `contracts/graphql/governance.json`.
