# ADR 0001 - Provider Billing V1

## Statut

Accepte.

## Date

2026-05-05

## Decision

nvbes Drive V1 utilise Stripe comme provider billing cible.

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

- Les entites billing doivent mapper les identifiants Stripe.
- Les webhooks Stripe doivent etre signes, idempotents et audites.
- Les prix Stripe doivent etre versionnes.
- La fiscalite EU doit etre validee avant lancement payant.
