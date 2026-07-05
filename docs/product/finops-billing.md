# FinOps et Billing

## Objectif

Ce document definit les exigences economiques, FinOps, Stripe et fiscales minimales pour nvbes Drive V1.

Le produit ne doit pas seulement etre techniquement fiable: il doit rester rentable a mesure que le stockage, l'egress, les logs, les backups et le support augmentent.

Audit complementaire:

- [Audit FinOps: trajectoire production à coût fixe quasi nul](finops-zero-cost-production-audit.md)

## Hypothese Billing V1

Stripe est le provider billing cible pour la V1.

Objets Stripe a mapper:

- Product.
- Price.
- Customer.
- Subscription.
- SubscriptionItem.
- Checkout Session.
- Customer Portal.
- Invoice.
- Tax.
- Meter ou usage event selon le modele Stripe disponible au moment de l'implementation.
- Webhook Event.

Regles:

- nvbes garde une source interne auditable des usages avant envoi a Stripe.
- Le ledger interne nvbes est canonique; Stripe et Mollie sont des providers d'execution et de reconciliation.
- Stripe reste provider d'execution V1 pour checkout, paiement, portail et webhooks tant que la plateforme interne monte en maturite.
- nvbes est la source de verite pour droits produit, quotas, usages et audit.
- Les invoices, payments, refunds, credit notes, write-offs et adjustments doivent etre reconstructibles depuis les tables canoniques nvbes.

## Unit Economics

Chaque plan doit avoir une marge brute cible avant lancement public.

Couts a modeliser par workspace:

- Object Storage principal.
- Operations Object Storage: PUT, GET, LIST, HEAD, multipart.
- Bande passante sortante.
- Backups PostgreSQL.
- Versioning ou lifecycle Object Storage.
- Logs et retention logs.
- Anti-malware et quarantaine.
- Compute API.
- Compute workers.
- PostgreSQL.
- Monitoring/alerting.
- Emails transactionnels.
- Frais Stripe.
- Taxes non recuperees si applicables.
- Support client estime.

Marge cible V1:

- Marge brute minimale cible: 70% par plan mature.
- Marge brute minimale acceptable au lancement: 50% si le plan est limite par quotas et surveille.
- Aucun plan ne doit etre lance sans estimation de cout dans un scenario normal et un scenario heavy user.

## Plans et Garde-Fous

Les volumes inclus sont des limites commerciales, pas des promesses de cout illimite.

Chaque plan doit definir:

- Stockage inclus.
- Utilisateurs inclus.
- Bande passante incluse.
- Taille max par fichier.
- Nombre max de liens actifs.
- Nombre max d'uploads par heure/jour si necessaire.
- Retention incluse.
- Limite de trial.
- Politique de depassement.

En cas de depassement:

- Alerte client avant blocage.
- Grace period courte pour eviter une coupure brutale.
- Blocage des nouveaux uploads si quota critique depasse.
- Downloads existants maintenus sauf abus ou incident.
- Upgrade propose dans le produit.

## Pay-As-You-Use V1

Meters V1:

| Meter              | Unite            | Frequence               | Facturation V1              |
| ------------------ | ---------------- | ----------------------- | --------------------------- |
| `storage_gb_month` | Go-mois          | snapshot quotidien      | facture mensuelle           |
| `team_seat_month`  | utilisateur-mois | prorata subscription    | facture mensuelle           |
| `egress_gb`        | Go sortant       | aggregation journaliere | inclus puis surveille en V1 |

Meters plus tard:

| Meter                | Unite   | Usage                  |
| -------------------- | ------- | ---------------------- |
| `retention_gb_month` | Go-mois | retention etendue      |
| `ocr_page`           | page    | OCR                    |
| `ai_credit`          | credit  | recherche IA ou resume |
| `archive_gb_month`   | Go-mois | archivage long terme   |

Regles:

- Arrondi stockage au Go superieur pour la facturation.
- Snapshot quotidien conserve pour audit.
- Usage envoye a Stripe seulement apres validation interne.
- Estimation de facture visible dans le produit.
- Alerte a 80% et 100% de quota.
- Cap de depense configurable plus tard pour Workspace/Business.

## Trial et Anti-Abus Economique

Essai recommande:

- 14 jours.
- Sans carte bancaire.
- 5 Go maximum.

Protections:

- Verification email obligatoire.
- Limite de workspaces par email, domaine et IP.
- Blocage ou friction sur domaines email jetables.
- Limite d'uploads par heure/jour pendant trial.
- Limite d'egress pendant trial.
- Pas de liens publics illimites pendant trial.
- Suppression ou archivage automatique des workspaces trial expires selon politique de retention.
- Monitoring des trials couteux.
- Upgrade obligatoire pour debloquer quotas complets.

## TVA et Facturation EU

Le pricing public doit preciser si les prix sont HT ou TTC.

V1 doit couvrir:

- Pays client.
- B2B vs B2C.
- Numero de TVA intracommunautaire pour clients B2B.
- Reverse charge quand applicable.
- Stripe Tax ou mecanisme equivalent.
- Factures conformes avec mentions legales.
- Conservation des factures selon obligations comptables.
- Adresse de facturation.
- Devise principale: EUR.

Regle:

- Aucun lancement payant EU sans validation fiscale/comptable minimale.
- Les calculs TVA, l'e-invoicing, la revenue recognition auditee et la retention des pieces comptables restent sous validation expert-comptable/fiscaliste avant activation multi-pays.

## Objectifs Business V1

Targets initiales a valider:

- Activation: 60% des nouveaux workspaces uploadent un fichier en moins de 10 minutes.
- Conversion trial to paid: 10% minimum au lancement, cible 20%.
- Marge brute: 50% minimum au lancement, cible 70%.
- Churn mensuel logo: cible sous 5% apres les premiers clients.
- ARPA personnel et groupe suivis separement.
- Cout infra par workspace actif: suivi hebdomadaire.
- Cout par To stocke: suivi mensuel.
- MRR cible premiere validation: 1 000 EUR.
- Seuil de rentabilite produit: a calculer avant lancement payant avec couts fixes et variables.

## FinOps Observabilite

Metriques a suivre:

- Cout par workspace.
- Cout par plan.
- Cout par To stocke.
- Egress par workspace.
- Operations Object Storage par workspace.
- Volume logs par environnement.
- Cout backups.
- Cout anti-malware.
- Cout compute API.
- Cout workers.
- Cout PostgreSQL.
- Frais Stripe.
- Support estime par workspace.

Alertes:

- Budget mensuel cloud depasse 80%.
- Budget mensuel cloud depasse 100%.
- Workspace trial au-dessus du cout attendu.
- Workspace payant avec marge brute negative.
- Egress anormal.
- Operations Object Storage anormales.
- Logs en croissance anormale.

## Gouvernance Billing

- Toute modification de prix ou quota passe par review.
- Les changements catalogue/prix nvbes sont versionnes; les objets Stripe/Mollie sont des miroirs ou references provider.
- Les anciens prix restent supportes ou migration documentee.
- Les refunds, credits et adjustments sont audites.
- Les erreurs billing SEV2 ou plus ont postmortem.
