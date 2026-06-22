# ADR 0001 - Provider Billing V1

## Statut

Accepte.

## Date

2026-05-05

## Decision

nvbes Drive V1 utilise Stripe comme provider billing cible.

Cette decision est limitee au provider d'execution V1. Elle ne fait pas de Stripe la source de verite long terme pour le modele produit, les entitlements, les subscriptions canoniques, les invoices nvbes, le ledger interne ou les rapports finance. La direction cible est definie par [ADR 0003](0003-internal-billing-platform.md).

Stripe gere:

- Paiements.
- Checkout.
- Customer Portal.
- Subscriptions.
- Invoices.
- Taxes via Stripe Tax ou mecanisme equivalent.
- Webhooks billing.

nvbes garde une source interne auditable pour:

- Quotas.
- Usages.
- Droits produit.
- Estimations de facture.
- Audit billing.

## Raisons

- Stripe reduit le risque d'implementation paiement.
- Stripe supporte subscriptions, factures et portail client.
- Le produit a besoin d'un modele package + usage.
- nvbes doit quand meme garder son propre ledger d'usage pour controler marge, quotas et audit.

## Consequences

- Les entites billing legacy peuvent mapper les identifiants Stripe; les nouvelles entites canoniques utilisent des mappings provider-neutral.
- Les webhooks Stripe doivent etre signes, idempotents et audites.
- Les prix Stripe doivent etre versionnes.
- La fiscalite EU doit etre validee avant lancement payant.
- Aucun nouveau domaine produit ne doit dependre directement d'un champ `stripe_*`.
