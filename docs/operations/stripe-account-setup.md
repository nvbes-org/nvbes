# Configuration Stripe et lancement de nouveaux produits

Date de référence : 2026-09-07. Procédure proposée, pas attestation de conformité
ni preuve que les réglages ont été appliqués dans Stripe.

## Périmètre actuel

L'exploitant est Rayane Guemmoud EI, SIREN 905 331 674. Le nom commercial
enregistré reste OTAKIMI tant que sa modification n'est pas effective. Nvbes
désigne le service et la présentation publique envisagée.

La [direction V1](../product/nvbes-product-strategy.md) reste applicable :
tests uniquement, aucun lancement payant implicite. Le runtime Billing refuse
les clés live dans [sa configuration](../../apps/billing-service/src/billing.config.rs).
Cette protection ne doit pas être supprimée pour configurer Stripe.

## Structure recommandée

| Objet | Usage pour Nvbes |
| --- | --- |
| Entité juridique | Rayane Guemmoud EI, France ; informations conformes aux justificatifs |
| Organisation | Facultative avec un compte ; nom interne proposé « Rayane Guemmoud EI » ; utile pour plusieurs activités indépendantes |
| Compte marchand | « Nvbes » pour l'activité Nvbes ; vérifier et réutiliser l'existant avant toute création |
| Sandboxes | « nvbes-dev » pour développement, « nvbes-staging » pour recette reproductible |
| Account groups | Filtres de reporting entre comptes, facultatifs ; pas une frontière de sécurité |
| Sharing groups | Partage de clients et moyens de paiement : désactivé par défaut |
| Connect | Hors besoin actuel : les utilisateurs et équipes Nvbes ne sont pas des marchands tiers |
| Product / Price | Offres vendues et tarifs versionnés à l'intérieur du compte marchand |

Une organisation n'encaisse pas elle-même. Chaque compte conserve ses données
et son activité. Un account group peut contenir plusieurs comptes et un compte
peut appartenir à plusieurs groupes. Il ne sépare pas les produits d'un compte.

Pour un nouveau tarif ou une offre de la même activité, réutiliser le compte.
Pour un site, projet ou métier exploité indépendamment, Stripe demande un compte
distinct. Plusieurs comptes peuvent reprendre les mêmes données d'EI. Regrouper
ces comptes dans une organisation si cela simplifie effectivement leur gestion.

## Réglages et conformité avant encaissement

1. **Vérification de l'entreprise** : type EI/micro-entrepreneur correspondant
   au parcours français, identité civile, SIREN, activité réelle, justificatifs
   et adresse complète demandés par Stripe. Ne pas remplacer cette adresse
   par l'adresse abrégée des projets de documents légaux. La diffusion partielle
   Sirene ne dispense pas du KYC Stripe.
2. **Présentation publique** : Nvbes, URL du produit réellement accessible,
   description précise, contact support suivi et libellé bancaire reconnaissable
   validé par Stripe. Vérifier les données visibles dans Checkout, reçus, portail
   et factures ; ne pas présumer que toutes les adresses restent privées.
3. **Documents B2C** : identité du vendeur, mentions légales complètes, CGV
   adaptées au produit payant, confidentialité, prix et devise, fourniture,
   rétractation, résiliation, garanties et médiateur applicable. Les modèles
   actuels hors V1 et l'adresse abrégée non validée ne constituent pas un dossier
   prêt pour le lancement.
4. **Fiscalité** : confirmer franchise ou assujettissement, pays de vente,
   catégorie fiscale du service et traitement TVA avant de paramétrer les taux.
   Ni 20 % partout ni exemption automatique du fait du régime micro. Stripe Tax
   exige un paramétrage et des obligations fiscales validés ; son activation
   seule ne remplit pas les obligations d'immatriculation, déclaration et paiement.
5. **Facturation** : selon l'[ADR Billing](../adr/0003-internal-billing-platform.md),
   Billing détient le modèle canonique. Définir explicitement l'émetteur fiscal,
   la numérotation, les avoirs, l'archivage et la correspondance avec Stripe avant
   activation. Éviter deux factures fiscales pour la même vente. Vérifier aussi
   les obligations françaises de facturation électronique et de e-reporting.
6. **Accès** : compte nominatif, passkey ou clé de sécurité, accès minimaux ;
   pas de clé d'organisation large dans le runtime. Clés restreintes par
   environnement, droits validés par essais, secrets hors Git et hors journaux.
7. **FinOps** : chiffrer frais Payments, Billing/Invoicing et options éventuelles
   (Tax, Radar, Sigma, domaines personnalisés). Aucun abonnement supplémentaire
   par défaut ; plafond global récurrent de 30 EUR TTC/mois.

Le partage de clients/cartes entre comptes exige le consentement prévu par
Stripe. Son activation n'est pas réversible en autonomie : contacter Stripe
pour désactiver. Ne pas le confondre avec les account groups de reporting.

## Dashboard et CLI

Utiliser le Dashboard pour les organisations, comptes marchands, account groups,
KYC, banque et gestion des sandboxes rattachées au compte. Ne pas utiliser
`stripe accounts create` comme raccourci : cette API concerne les comptes Connect.

La CLI pilote les ressources API, le catalogue, les lectures de configuration
et les tests. Les profils CLI sont des alias locaux ; leurs noms ne garantissent
pas l'environnement réellement sélectionné. Confirmer le compte dans le Dashboard
et l'identifiant retourné par l'API avant toute écriture.

Connexion, en choisissant la sandbox correspondante dans le navigateur :

```bash
rtk stripe login --project-name nvbes-dev
rtk stripe login --project-name nvbes-staging
rtk stripe get /v1/account --project-name nvbes-dev
rtk stripe get /v1/products --project-name nvbes-dev --limit 100
rtk stripe get /v1/prices --project-name nvbes-dev --limit 100
rtk stripe get /v1/webhook_endpoints --project-name nvbes-dev --limit 100
```

Paginer si `has_more=true`. Les réponses peuvent contenir des informations
internes : ne pas les publier brutes. Ne jamais afficher `stripe config --list`
sans filtrage, ni copier une clé dans une commande versionnée.

La CLI 1.50.10 fournit aussi `login --non-interactive`, qui retourne un lien,
un code et une commande de finalisation à suivre. Les clés CLI ne remplacent
pas les secrets durables du runtime ou de la CI.

## Procédure pour chaque nouveau produit

1. Documenter l'offre : service rendu, public, pays, prix, périodicité,
   résiliation, fiscalité et coût total. Valider sa sortie du périmètre V1.
2. Décider compte existant ou activité indépendante ; consigner l'identifiant
   du compte cible et des sandboxes, sans secrets.
3. Définir le catalogue canonique côté Billing et ses identifiants stables.
   Projeter un Product et des Prices dans Stripe ; enregistrer les mappings
   par fournisseur, compte et environnement.
4. Versionner les tarifs : nouvelle Price pour un nouveau montant ou cycle ;
   conserver l'historique et décider explicitement du sort des abonnés existants.
5. Tester en sandbox dev puis staging : achat réel de bout en bout en test,
   3DS, refus, renouvellement, impayé, annulation, remboursement, webhook retardé,
   doublonné ou désordonné. Une fixture CLI seule ne prouve pas les droits Nvbes.
6. Vérifier les signatures, l'idempotence, les montants côté serveur et la
   réconciliation. Ne pas attribuer un accès sur la seule redirection Checkout.
7. Avant live, valider conformité, frais, rollback, API versionnée et prise en
   charge réelle des événements. Le code V1 actuel interdit encore les clés live.
8. Recréer explicitement les ressources approuvées dans le compte live lors
   d'un lancement autorisé ; les identifiants test ne deviennent pas des IDs live.
   Vérifier les nouveaux mappings avant d'ouvrir les ventes.

Exemple purement illustratif : aperçu local des requêtes, sans appel de création.
Le montant de 5 EUR n'est pas une décision de tarif ou de fiscalité Nvbes.

```bash
rtk stripe products create --project-name nvbes-dev --dry-run \
  --name 'Nvbes Exemple' -d 'metadata[product_code]=example'
rtk stripe prices create --project-name nvbes-dev --dry-run \
  --product prod_REPLACE --currency eur --unit-amount 500 \
  -d 'recurring[interval]=month' -d 'lookup_key=example_monthly_eur_v1'
```

Le lancement réel de ces commandes exige des valeurs approuvées, sans `--dry-run`,
et une clé d'idempotence stable par opération. Ne pas ajouter `--live` aux essais.

Pour le serveur lancé avec `pnpm dev:billing-service` (port local par défaut 3080) :

```bash
rtk stripe listen --project-name nvbes-dev \
  --forward-to http://127.0.0.1:3080/webhooks/stripe
```

Placer le secret de signature de cette session dans `NVBES_STRIPE_WEBHOOK_SECRET`
localement et redémarrer le serveur. Il est distinct du secret d'une destination
webhook Dashboard. `NVBES_STRIPE_SECRET_KEY` doit cibler la même sandbox.
Limiter les événements de la destination déployée aux événements réellement
traités et épingler leur version API après tests de compatibilité.

## Sources officielles

- [Organisation et groupes](https://docs.stripe.com/get-started/account/orgs/build)
- [Comptes indépendants](https://docs.stripe.com/get-started/account/multiple-accounts)
- [Gestion des sandboxes](https://docs.stripe.com/sandboxes/dashboard/manage)
- [Partage de clients et moyens de paiement](https://docs.stripe.com/get-started/account/orgs/sharing/customers-payment-methods)
- [Vérification et informations publiques](https://docs.stripe.com/get-started/account/set-up)
- [Exigences du site](https://docs.stripe.com/get-started/checklist/website)
- [Stripe Tax](https://docs.stripe.com/tax)
