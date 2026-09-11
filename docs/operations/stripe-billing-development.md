# Stripe dans Billing : local, CI et production

`pnpm dev` synchronise `.env` sans écraser les valeurs existantes, vérifie
l'accès à la sandbox puis lance les services. `pnpm dev:billing-service` lance
Billing seul. PostgreSQL doit être disponible pour ce lancement isolé.

## Premier démarrage local

Prérequis : Node, Cargo, PostgreSQL (`psql`), Docker pour le démarrage complet,
et Stripe CLI. La CLI doit proposer `stripe listen`.

Dans `.env` (ignoré par Git, permissions 0600) :

```dotenv
NVBES_STRIPE_DEV_MODE="sandbox"
NVBES_STRIPE_ACCOUNT_ID="acct_IDENTIFIANT_DE_LA_SANDBOX"
NVBES_STRIPE_SECRET_KEY=""
NVBES_STRIPE_CLI_PROJECT="nvbes-dev"
NVBES_APP_URL="http://localhost:3001"
```

Renseigner la clé restreinte de cette sandbox dans `NVBES_STRIPE_SECRET_KEY`.
Permissions du setup local : Balance et Events en lecture ; Customers,
Checkout Sessions, Customer Portal, Products, Prices et Stripe CLI en écriture.
Products/Prices en écriture servent uniquement à préparer l'offre de test.
La clé CI est distincte ; une future clé du serveur déployé n'a pas besoin de
ces permissions de provisioning ni de Stripe CLI.

Le lanceur refuse les clés live et les bases distantes ou non dédiées au dev.
Il vérifie le solde en mode test avec le contexte `Stripe-Account` attendu,
migre la base, crée ou retrouve une offre `sandbox_monthly` à 1 EUR/mois
**fictive**, puis enregistre son `price_…` dans la base locale. Les anciens
prix fictifs `price_test_…` sont désactivés dans cette base.

Il lance ensuite `stripe listen` vers `127.0.0.1:3080/webhooks/stripe`, récupère
le `whsec_…` en mémoire et le transmet uniquement au processus Billing.
La clé API est transmise à la CLI par environnement ; aucun secret n'est placé
dans les arguments de commande ou dans les logs du listener. À l'arrêt, le
listener et le backend sont arrêtés ensemble. Un échec du listener arrête Billing.

Le profil CLI ne remplace pas une clé applicative : la session OAuth personnelle
n'est pas utilisée comme secret de serveur ou de CI. Le listener lancé par le
setup utilise explicitement la clé restreinte de `.env`.

Pour travailler hors ligne, choisir explicitement
`NVBES_STRIPE_DEV_MODE="mock"`. Le setup ne bascule jamais silencieusement en mock
après une erreur Stripe. L'offre sandbox reste distincte des offres commerciales.

## GitHub Actions

Le workflow `Stripe sandbox integration` est manuel, limité à `main`, avec un
timeout de 20 minutes et une seule exécution à la fois. L'environnement GitHub
`stripe-ci` contient :

- secret `STRIPE_CI_SECRET_KEY` : clé restreinte de test dédiée à la CI ;
- variable `STRIPE_ACCOUNT_ID` : compte sandbox attendu.

Configuration actuelle : `Nvbes sandbox` (`acct_1UD3lLIXHnmBruPP`) est partagée
entre local et CI, avec deux clés distinctes : `nvbes-local-billing` et
`nvbes-github-ci`. La clé CI dispose de Balance en lecture et Customers,
Products, Prices, Checkout Sessions en écriture. Elle n'a pas accès à Stripe CLI.

Il compile Billing, exécute les régressions PostgreSQL puis le smoke Stripe.
Le smoke utilise une base éphémère, crée une session Checkout réelle **en sandbox**,
vérifie son idempotence, simule des webhooks signés et leurs répétitions, vérifie
le refus du live puis expire sa session Checkout. Il n'effectue aucun paiement.
Les clients de test portent des UUID distincts ; aucun nettoyage global de la
sandbox n'est effectué. Une sandbox distincte pourra remplacer ce partage lorsque
Stripe permettra sa création.

Les tests de PR ordinaires utilisent les fixtures locales sans secrets Stripe.
Ce workflow manuel n'est pas une preuve de paiement 3DS ou de renouvellement
d'abonnement : ces scénarios nécessitent une validation complémentaire avant live.

## Serveur déployé et production payante

Un serveur déployé reçoit une clé propre à son environnement et un secret
de destination webhook HTTPS permanente (`/webhooks/stripe`). Il ne lance pas
`stripe listen`. Le secret de la CLI locale n'est pas celui de cette destination.

Le workflow `deploy.yml` (job `deploy-billing`) utilise déjà l'environnement `production-billing`
et les secrets `STRIPE_SECRET_KEY` / `STRIPE_WEBHOOK_SECRET`. Déploiement du
serveur et encaissement live restent deux décisions différentes : la V1 refuse
toujours les clés live et les événements `livemode=true`.

Chaque base conserve ses propres prix, clients et abonnements Stripe. Ne pas
copier les identifiants de sandbox vers une base destinée au live. L'activation
live nécessite un changement explicite de la politique V1, les contrôles de
facturation/fiscalité et les tests du cycle complet de paiement.
