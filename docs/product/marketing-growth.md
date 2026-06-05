# Marketing, Conversion et Product Analytics

## Objectif

Ce document definit la strategie marketing V1, le funnel de conversion, les objectifs cles, les KPIs, l'observabilite produit et les contraintes analytics RGPD.

## ICP de Lancement

ICP prioritaire V1:

- Agences et studios de 2 a 10 personnes.
- Travaillent avec des clients externes.
- Partagent regulierement des fichiers sensibles ou livrables.
- Ont besoin de liens de partage controlables.
- Veulent une solution simple, europeenne et plus professionnelle qu'un drive personnel.

Segments secondaires:

- Cabinets de conseil.
- Independants premium avec clients recurrents.
- Associations structurees manipulant des documents sensibles.

Segments non prioritaires V1:

- Grand public B2C.
- Grandes entreprises avec besoins SSO/SAML.
- Equipes qui cherchent une suite bureautique collaborative.

## Pains Prioritaires

- Les fichiers clients sont disperses entre outils personnels.
- Les liens de partage restent actifs trop longtemps.
- L'equipe ne sait pas quels fichiers sont exposes.
- Les outils americains posent une question de confiance ou de souverainete.
- La facturation et les quotas sont peu lisibles.
- Les petites equipes veulent du controle sans complexite enterprise.

## Alternatives

- Google Drive.
- Dropbox.
- OneDrive.
- WeTransfer pour partage ponctuel.
- NAS ou serveur interne.
- Object storage brut pour profils techniques.

## Objections

- "Pourquoi ne pas garder Google Drive ?"
- "Est-ce assez fiable pour mes fichiers clients ?"
- "Ou sont stockees les donnees ?"
- "Est-ce vraiment simple pour mon equipe ?"
- "Combien vais-je payer si j'utilise plus de stockage ?"
- "Puis-je recuperer ou supprimer mes donnees ?"

## Positionnement Marketing

Message principal:

```text
Le drive europeen securise pour les petites equipes qui partagent des fichiers clients.
```

Sous-message:

```text
Stockez, organisez et partagez vos fichiers dans un espace clair, controle et heberge en Europe.
```

Preuves a mettre en avant:

- Hebergement europeen.
- Liens avec expiration obligatoire.
- Vue des liens actifs.
- Roles d'equipe.
- Facturation lisible.
- Export et suppression des donnees.
- Chiffrement au repos.
- API publique pour automatiser les workflows fichiers.

## Canal de Lancement

Canal principal V1:

- Outreach direct vers agences, studios et cabinets de conseil.

Canaux secondaires:

- SEO sur requetes "drive europeen", "partage fichier securise", "alternative Google Drive europe".
- Contenu comparatif.
- Reseau fondateur.
- Communautes de freelances/agences.
- Partenariats plus tard.

## Funnel V1

| Etape             | Definition                           | Event                                 |
| ----------------- | ------------------------------------ | ------------------------------------- |
| Visitor           | Visite landing page                  | `marketing.page_viewed`               |
| Pricing viewed    | Consulte pricing                     | `marketing.pricing_viewed`            |
| Signup started    | Commence inscription                 | `auth.signup_started`                 |
| Signup completed  | Compte cree                          | `auth.signup_completed`               |
| Email verified    | Email verifie                        | `auth.email_verified`                 |
| Workspace created | Espace cree                          | `workspace.created`                   |
| First upload      | Premier fichier actif                | `activation.first_file_uploaded`      |
| First share link  | Premier lien cree                    | `activation.first_share_link_created` |
| Member invited    | Premier membre invite                | `activation.first_member_invited`     |
| Activated         | Upload + lien ou invitation sous 24h | `activation.workspace_activated`      |
| Trial ending      | Trial proche expiration              | `billing.trial_ending`                |
| Checkout started  | Checkout lance                       | `billing.checkout_started`            |
| Paid conversion   | Subscription active                  | `billing.subscription_activated`      |
| Retained          | Workspace actif a J30                | `retention.workspace_retained_30d`    |

## North Star Metric

North Star V1:

```text
Nombre de workspaces actifs qui partagent au moins un fichier avec controle de lien sur 30 jours.
```

Raison:

- Mesure l'usage coeur du produit.
- Relie stockage, collaboration et securite.
- Evite de mesurer uniquement le volume de stockage.

## Objectifs Cles V1

Objectif 1: valider l'activation.

- Owner: Produit.
- Target: 60% des nouveaux workspaces uploadent un fichier en moins de 10 minutes.
- Go/no-go: sous 30% apres 20 trials qualifies, revoir onboarding.

Objectif 2: valider la conversion.

- Owner: Growth.
- Target: 10% trial-to-paid minimum, cible 20%.
- Go/no-go: sous 5% apres 50 trials qualifies, revoir ICP, pricing ou promesse.

Objectif 3: valider la rentabilite.

- Owner: FinOps.
- Target: marge brute 50% minimum au lancement, cible 70%.
- Go/no-go: plan ou quota a revoir si un plan devient structurellement negatif.

Objectif 4: valider la retention.

- Owner: Produit.
- Target: churn logo mensuel sous 5% apres premiers clients.
- Go/no-go: churn superieur a 10% sur 2 mois, revoir onboarding equipe et valeur recurrente.

Cadence:

- Revue hebdomadaire pendant beta.
- Revue mensuelle apres lancement payant.

## Definitions KPI

Activation:

- Numerateur: workspaces qui uploadent au moins un fichier actif dans les 10 minutes suivant creation workspace.
- Denominateur: workspaces crees hors comptes internes/test.
- Source: product analytics + StorageObject.
- Segmentation: ICP, canal, plan choisi, pays.

Conversion trial-to-paid:

- Numerateur: workspaces trial devenus subscription active.
- Denominateur: workspaces trial qualifies.
- Periode: par cohorte hebdomadaire.
- Exclusions: comptes internes, fraudes, trials sans email verifie.
- Source: Stripe + ledger interne.

MRR:

- Somme des subscriptions actives normalisees mensuellement.
- Exclure taxes.
- Exclure credits/refunds ponctuels.
- Source: Stripe + BillingAccount.

ARPA:

- MRR / nombre de comptes payants actifs.
- Segmentation par plan et ICP.

Churn logo:

- Workspaces payants annules ou non renouveles / workspaces payants actifs au debut de periode.
- Periode: mensuelle.

Marge brute:

- Revenu hors taxes moins couts variables directs / revenu hors taxes.
- Couts variables: storage, egress, operations, backups, logs, anti-malware, Stripe, support estime.

## Event Taxonomy V1

Principes:

- Events au niveau workspace quand possible.
- Ne pas envoyer de nom de fichier, contenu, object key ou token.
- Utiliser IDs internes pseudonymises.
- Separer product analytics, audit logs et logs techniques.

Properties communes:

- `workspace_id`
- `user_id`
- `plan_code`
- `workspace_type`
- `role`
- `country`
- `source`
- `campaign`
- `created_at`

Events minimum:

- `marketing.page_viewed`
- `marketing.cta_clicked`
- `marketing.pricing_viewed`
- `auth.signup_started`
- `auth.signup_completed`
- `auth.email_verified`
- `workspace.created`
- `file.upload_started`
- `file.upload_completed`
- `activation.first_file_uploaded`
- `share_link.created`
- `activation.first_share_link_created`
- `member.invited`
- `activation.first_member_invited`
- `activation.workspace_activated`
- `billing.checkout_started`
- `billing.subscription_activated`
- `billing.trial_ending`
- `billing.payment_failed`
- `retention.workspace_active_weekly`
- `retention.workspace_retained_30d`

## Dashboards V1

Dashboard Marketing:

- Visitors.
- Signup conversion.
- Pricing page conversion.
- CTA clicks.
- Source/campaign performance.

Dashboard Activation:

- Signup to workspace created.
- Workspace created to first upload.
- First upload to first share link.
- First upload to member invited.
- Time to first upload.

Dashboard Revenue:

- Trial-to-paid.
- MRR.
- ARPA.
- Churn.
- Plan mix.
- Payment failures.

Dashboard Retention:

- Weekly active workspaces.
- Active sharing workspaces.
- Members per workspace.
- Share links per active workspace.
- J30 retention.

Dashboard FinOps:

- Marge brute par plan.
- Cout par workspace.
- Cout par To stocke.
- Egress par workspace.
- Trials couteux.

## Analytics RGPD

Regles:

- Consentement explicite pour analytics marketing non essentiels.
- Product analytics limite aux evenements necessaires a l'amelioration du service.
- Opt-out documente.
- Minimisation stricte des properties.
- Pas de noms de fichiers, contenu, emails en clair, tokens ou object keys dans les analytics.
- Retention analytics documentee.
- Attribution marketing sans fingerprinting invasif.
- DPA et sous-traitants analytics documentes.
- Possibilite d'export/suppression des donnees analytics rattachees a un utilisateur si applicable.

## Landing Page V1

Hero:

```text
Le drive europeen securise pour les petites equipes qui partagent des fichiers clients.
```

Sous-titre:

```text
Un espace clair pour stocker, organiser et partager vos fichiers avec expiration des liens, roles d'equipe et donnees hebergees en Europe.
```

CTA principal:

```text
Creer mon espace
```

CTA secondaire:

```text
Voir les offres
```

Sections:

- Probleme: fichiers disperses, liens oublies, manque de controle.
- Solution: espace equipe, liens expires, vue des liens actifs.
- Securite: hebergement europeen, chiffrement au repos, audit basique.
- API: automatiser uploads, liens de partage et workflows fichiers.
- Pricing: Solo Pro, Team, Team Plus.
- FAQ objections.
- CTA final.

FAQ V1:

- Ou sont hebergees les donnees ?
- Puis-je revoquer un lien partage ?
- Que se passe-t-il si je depasse mon quota ?
- Puis-je connecter nvbes Drive a mes outils internes ?
- Est-ce une alternative complete a Google Drive ?
- Puis-je exporter ou supprimer mes donnees ?

## Retention et Expansion

Boucles V1:

- Invitation membre apres premier upload.
- Creation de lien securise apres premier fichier.
- Rappel trial ending.
- Alerte quota a 80% et 100%.
- Vue liens actifs pour encourager controle regulier.
- Usage summary hebdomadaire plus tard.
- Upgrade prompt quand stockage ou seats atteignent les limites.
- Relance inactive si aucun fichier importe sous 48h.

Moments d'upgrade:

- Quota stockage proche limite.
- Besoin d'ajouter un membre.
- Besoin de retention plus longue.
- Usage recurrent de liens partages.

## Experiment Log

Chaque test marketing ou conversion doit documenter:

- Hypothese.
- Segment cible.
- Changement teste.
- KPI principal.
- Date debut/fin.
- Resultat.
- Decision.
