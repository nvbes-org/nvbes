# ADR 0004 - Routing PSP Provider-Neutral et Fallback Regional

## Statut

Propose.

## Date

2026-07-02

## Decision

nvbes route les paiements via une couche provider-neutral qui choisit un provider selon la residence des donnees, l'eligibilite operationnelle, les frais visibles, la probabilite de succes et les regles produit explicites.

La preference cible est locale ou regionale quand elle est compatible avec le produit:

- CB via un acquereur ou PAT agree CB pour les flux carte France et Europe quand le perimetre contractuel le permet.
- Mollie comme PSP europeen principal pour les abonnements et paiements recurrents eligibles.
- Stripe comme provider externe de fallback seulement quand il est active explicitement par configuration et par routing rule.

Drive reste Stripe-only tant qu'il n'a pas d'adapter Mollie complet. Cette restriction est exposee comme politique produit dans les decisions de routing et dans l'audit, au lieu d'etre un blocage implicite.

## Contexte

nvbes doit reduire le vendor lock-in paiement, eviter les dependances fortes aux entreprises americaines, et garder des backups locaux ou regionaux. Le systeme doit aussi maximiser le taux de succes sans contourner les contraintes de residence, de cout, d'audit et de conformite.

CB apporte des capacites utiles pour le marche francais:

- Fast'R by CB pour securiser l'e-commerce carte.
- Safe'R by CB pour les paiements CIT eligibles avec friction reduite, hors subscriptions et recurring.
- Updat'R by CB pour maintenir a jour les cartes enregistrees et limiter les interruptions d'abonnement ou d'achat en un clic.

Ces services CB ne remplacent pas un modele provider-neutral: ils doivent etre branches via un acteur agree et rester derriere les memes ports PSP que Mollie et Stripe.

## Regles d'architecture

- Les domaines produit ne dependent pas d'un provider specifique.
- Les IDs provider sont stockes via mappings provider-neutral; les champs legacy `stripe_*` restent de compatibilite et ne sont jamais ecrases par Mollie.
- Chaque decision de routing conserve le provider choisi, la raison, les candidats, le statut operationnel, la politique produit et les flags de fallback.
- Le fallback externe est desactive par defaut et exige une configuration explicite.
- Les webhooks provider sont idempotents, retenus comme evenements provider, traites ou marques en erreur, puis rejouables par flux admin.
- nvbes ne collecte ni PAN ni CVV; les mandats, tokens et changements de moyen de paiement restent portes par le PSP ou l'acquereur.

## Consequences

- Mollie doit gerer le cycle customer, mandat, paiement initial et subscription recurrents pour etre un vrai provider d'abonnement.
- Stripe reste utile comme fallback mais ne peut plus etre une dependance implicite du modele produit.
- CB devient une cible d'integration future via acquereur/PAT, pas une dependance directe a encoder dans les produits.
- Le routing doit privilegier la conformite et la residence avant le cout minimal.
- Les operations admin doivent afficher les echecs provider, les differences de reconciliation, les retries et les decisions de routing avec assez de contexte pour l'audit.
