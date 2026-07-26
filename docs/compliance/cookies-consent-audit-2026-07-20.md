# Cartographie et audit cookies, traceurs et consentements

Date de revue: 20 juillet 2026

Périmètre revu:

- `apps/account-web`
- `apps/cloud-web`
- `libs/ts/web-runtime`
- API et persistance des consentements dans `apps/account-service`
- documentation légale et conformité du dépôt

Ce document est un audit technique de conformité et ne remplace pas une
validation juridique formelle.

## Synthèse exécutive

Le dispositif ne doit pas être déclaré conforme CNIL en l'état.

L'interface de premier niveau respecte plusieurs fondamentaux: aucun choix
optionnel par défaut, boutons `Tout refuser`, `Personnaliser` et `Tout accepter`
au même niveau, accès persistant aux préférences et durée locale d'environ six
mois. En revanche, le modèle réellement exécuté contredit la spécification
interne sur un point central: les finalités PostHog ne sont pas séparées.

Une acceptation de la catégorie « Analyse d'audience » active en bloc:

- product analytics;
- autocapture/heatmaps;
- session replay;
- surveys/feedback;
- feature flags/expérimentations.

Les contrôles granulaires existent dans les types, mais la fonction d'édition
granulaire est volontairement sans effet et la normalisation recalcule toutes
les finalités depuis la catégorie ou le fournisseur. La synchronisation depuis
le backend reproduit le même élargissement. Le consentement n'est donc pas
spécifique aux finalités réellement activées.

Autres risques majeurs:

- Sentry est initialisé par le transport analytics au démarrage lorsque son DSN
  est configuré, indépendamment du consentement, alors qu'un second chemin
  Sentry correctement conditionné au consentement coexiste dans
  `account-web`;
- une ancienne acceptation binaire `v1` est migrée vers l'acceptation de toutes
  les nouvelles finalités, y compris le session replay;
- la preuve backend est attachée au compte et non au terminal, puis le « dernier
  changement gagne » entre le navigateur et le backend, sans mécanisme
  explicite de consentement multi-terminal présenté à l'utilisateur;
- la preuve backend écrase les réacceptations par `UPSERT`, ce qui ne conserve
  pas l'historique complet des décisions dans la table principale;
- les finalités, durées, responsables et conséquences du choix ne sont pas
  suffisamment détaillés au moment du recueil;
- la politique de confidentialité est encore un template non publiable;
- aucun inventaire vérifiable des traceurs avec nom, domaine, finalité,
  fournisseur et durée n'a été trouvé.

Niveau de risque constaté lors de l'audit initial: **élevé avant mise en
production avec les clés PostHog/Sentry actives**.

## État de remédiation technique

Correctifs implémentés le 20 juillet 2026:

- finalités analytics devenues source de vérité;
- commandes par catégorie limitées à un rôle de sélection groupée;
- contrôles séparés pour les deux finalités réellement utilisées: analytics
  produit et rapports d'erreurs;
- heatmaps, replay, surveys et expérimentation laissés désactivés et absents de
  l'interface tant qu'aucun produit ne les utilise;
- synchronisation backend de chaque finalité au lieu de leur suppression;
- reconstruction backend limitée aux finalités explicitement actives;
- notice `cookie-notice-2026-07-20` et stockage
  `nvbes.tracking-consent.v4`;
- anciennes décisions locales et anciennes versions backend exclues de la
  nouvelle notice, afin d'imposer un nouveau choix;
- chargement dynamique de Sentry seulement après consentement à
  `errorTracking`;
- fermeture de Sentry au retrait, même si une autre finalité reste active;
- opt-out, reset et purge PostHog lorsque toutes ses finalités sont retirées;
- révocation d'une finalité backend répercutée sur la seule finalité locale
  concernée.

Restent hors correctif technique et nécessitent une décision juridique ou
organisationnelle:

- finalisation des mentions légales et de la politique de confidentialité;
- publication de l'inventaire contractuel des traceurs et durées effectivement
  configurées;
- décision formelle sur la portée multi-terminal;
- preuve append-only transactionnelle côté backend;
- validation des contrats, transferts et durées chez les fournisseurs.

## Cartographie fonctionnelle

### 1. Recueil dans le navigateur

| Élément          | Implémentation                                      | État                                       |
| ---------------- | --------------------------------------------------- | ------------------------------------------ |
| Bannière commune | `libs/ts/web-runtime/src/TrackingConsentBanner.tsx` | Montée dans Account et Cloud               |
| Choix initial    | Refus, personnalisation, acceptation                | Symétrie de premier niveau correcte        |
| État par défaut  | Essentiels actifs, options inactives                | Correct                                    |
| Réouverture      | Bouton fixe « Préférences Cookies »                 | Retrait accessible                         |
| Stockage         | `localStorage`, clé `nvbes.tracking-consent.v3`     | Choix, source, dates et périmètre          |
| Expiration       | 183 jours                                           | Cohérent avec la cible interne de six mois |
| Anciennes clés   | `v1`, `v2`, `v3`                                    | Migrations automatiques                    |

Le stockage de la préférence elle-même peut être nécessaire à la conservation
du choix. Il doit néanmoins être décrit dans l'inventaire des traceurs.

### 2. Modèle de décision

Le modèle contient trois niveaux simultanés:

1. catégories: `essentials`, `analytics`, `performance`;
2. fournisseurs: Stripe, Identity, Cloudflare, PostHog, Sentry, Grafana;
3. finalités analytics: product analytics, heatmaps, replay, surveys, error
   tracking, feature flags.

Ces trois niveaux ne sont pas indépendants. `deriveConsentState()` et
`analyticsConsentForState()` font de la catégorie ou du fournisseur la source
de vérité et réactivent toutes les sous-finalités. En conséquence:

- désactiver une sous-finalité seule est impossible;
- `toggleConsentAnalyticsPurpose()` ne modifie rien;
- activer PostHog revient à consentir à cinq finalités distinctes;
- activer la performance, Sentry ou Grafana active `errorTracking`;
- le backend reconstruit les mêmes consentements larges depuis les catégories
  ou les fournisseurs.

### 3. Exécution des traceurs

#### PostHog

PostHog est initialisé à la première opération autorisée, et non au chargement
sans consentement. La configuration désactive initialement autocapture,
pageviews automatiques et session recording. Les événements, identifiants et
groupes passent par des garde-fous de consentement et de pseudonymisation.

Points positifs:

- capture produit conditionnée à `productAnalytics`;
- replay conditionné à `sessionReplay` et bloqué sur les routes sensibles;
- allowlist d'événements;
- propriétés nettoyées;
- identifiants UUID pseudonymisés par HMAC;
- arrêt, opt-out, reset et purge lors du retrait complet.

Limite déterminante: ces garde-fous consultent un consentement dont les
sous-finalités ont déjà été artificiellement agrégées.

#### Sentry

Deux chemins coexistent:

- `apps/account-web/src/identity.error.reporting.ts` initialise Sentry après
  vérification du consentement et le ferme au retrait;
- `libs/ts/web-runtime/src/analytics.browser-transport.ts` appelle `Sentry.init`
  dès la création du transport si un DSN est fourni.

Dans `cloud-web`, le second chemin reçoit directement l'environnement complet,
dont le DSN Sentry. Cette initialisation précoce est incompatible avec la règle
interne « bloqué par défaut » tant qu'une exemption strictement technique n'est
pas documentée. Même si les captures explicites du runtime sont filtrées, les
intégrations par défaut et l'initialisation du SDK ne doivent pas être traitées
comme une preuve d'absence de lecture, écriture ou émission avant consentement.

#### Grafana

Grafana apparaît dans le modèle de consentement frontend, mais aucun SDK
navigateur Grafana ni traceur associé n'a été identifié dans le périmètre
revu. Présenter un fournisseur qui n'effectue pas d'opération côté terminal
brouille le caractère éclairé du choix. L'observabilité backend via Alloy ne
relève pas automatiquement du consentement cookies du navigateur.

### 4. Synchronisation compte/backend

Pour un utilisateur authentifié, les décisions locales sont synchronisées vers
`user_consents` via l'API Identity:

- type de consentement;
- version de document (`v3`);
- date d'octroi et date de révocation;
- IP tronquée;
- principal;
- événements d'audit avec user-agent.

Le mécanisme compare la date locale à la date du dernier changement backend:

- backend plus récent: il remplace l'état local;
- sinon: le local est répliqué vers le backend.

Ce comportement est un consentement lié au compte et potentiellement
multi-terminal. Il faut l'assumer comme tel dans l'information donnée avant le
choix. À défaut, un consentement donné sur un appareil peut activer des traceurs
sur un autre sans attente claire de l'utilisateur.

La table utilise une unicité `(principal_id, document_version, consent_type)` et
un `UPSERT` qui remplace `granted_at`, `revoked_at` et l'IP lors d'une nouvelle
acceptation. L'audit append-only peut apporter une partie de l'historique, mais
son écriture est « best effort »: une défaillance d'audit ne fait pas échouer
l'enregistrement. La preuve complète dépend donc de deux stockages qui peuvent
diverger.

### 5. Autres « consentements » à ne pas confondre

| Domaine                | Nature                                               | Base/objet                     |
| ---------------------- | ---------------------------------------------------- | ------------------------------ |
| Cookies et traceurs    | Autorisation ePrivacy/RGPD par finalité              | `user_consents`, état local v3 |
| Documents contractuels | Acceptation de CGU/politique à l'inscription         | `REGISTER_LEGAL_DOCUMENTS`     |
| OAuth/OIDC             | Autorisation d'accès d'une application à des scopes  | `oauth_consents`               |
| GPC                    | Signal d'opposition `Sec-GPC` enregistré côté compte | `gpc_opt_out`                  |

Ces décisions ont des objets juridiques différents. Elles ne doivent pas
partager un vocabulaire UI, une version générique ou une logique de révocation
qui laisserait croire qu'elles sont interchangeables.

## Audit CNIL et RGPD

### Critiques

#### C-01 — Consentement non spécifique par finalité

Preuves:

- la spécification exige six finalités PostHog séparées;
- `analyticsConsentForState()` les active en bloc;
- `toggleConsentAnalyticsPurpose()` est sans effet;
- la bannière n'offre qu'un interrupteur PostHog global;
- la reconstruction backend active toutes les finalités analytics lorsqu'une
  catégorie ou un ancien consentement PostHog est actif.

Impact: consentement insuffisamment spécifique et potentiellement non éclairé;
replay, heatmaps, surveys et expérimentation activables sur la base d'une
formulation générale de mesure d'audience.

Remédiation: choisir les finalités comme unique source de vérité. Les catégories
ne doivent être que des commandes UI « tout activer/désactiver dans ce groupe ».
Le fournisseur doit être une information, pas une base d'autorisation.

#### C-02 — Initialisation Sentry avant consentement

Preuve: `createBrowserAnalyticsTransport()` appelle `initSentry()` lors de sa
création; `cloud-web` lui transmet `import.meta.env`, et `initSentry()` n'évalue
aucun consentement.

Impact: risque de traitement ou de traceur avant consentement, absence de
maîtrise démontrable des intégrations automatiques.

Remédiation: supprimer Sentry du transport analytics commun. Instancier et
charger dynamiquement le SDK uniquement après la finalité `errorTracking`,
avec fermeture et purge au retrait. Si une exemption technique est revendiquée,
la documenter séparément avec configuration, nécessité, minimisation,
destinataires et durée.

#### C-03 — Migration v1 trop permissive

Preuve: une valeur historique `accepted` est convertie en toutes catégories,
tous fournisseurs et toutes finalités, y compris replay.

Impact: extension rétroactive d'un ancien consentement à des finalités plus
intrusives qui n'étaient pas nécessairement présentées au moment du choix.

Remédiation: migrer au maximum la finalité historique démontrable. Pour toute
nouvelle finalité, conserver `false` et redemander un choix.

### Élevés

#### H-01 — Preuve incomplète et historique écrasé

La table ne conserve pas chaque décision comme un événement immuable. La
réacceptation écrase la décision précédente. L'audit secondaire peut échouer
sans bloquer l'opération.

Remédiation: journal append-only de décisions avec `decision_id`, sujet ou
terminal pseudonyme, état complet, version du texte et de l'UI, finalités,
fournisseurs informés, source, horodatage serveur, contexte multi-terminal et
empreinte de la configuration déployée. Maintenir séparément une projection de
l'état courant.

#### H-02 — Consentement multi-terminal implicite

L'état attaché au compte peut remplacer le choix local d'un autre appareil. Le
flux ne présente pas clairement cette portée avant le choix.

Remédiation: décider explicitement entre consentement par terminal et
multi-terminal. Pour le multi-terminal, appliquer les recommandations CNIL
2026: information claire sur la portée, mécanisme cohérent, retrait simple et
gestion robuste des conflits. Ne pas utiliser un simple « dernier timestamp
gagne » sans journal de décision.

#### H-03 — Information de premier niveau trop vague

La bannière mentionne « mesurer l'audience » et « analyser les performances »
mais pas clairement les conséquences du replay, des heatmaps, des surveys, des
expérimentations, les responsables du traitement, ni un accès direct à une
politique cookies détaillée.

Remédiation: exposer des finalités compréhensibles avant le choix, une liste
accessible des responsables/fournisseurs et un lien direct vers la politique
cookies. Décrire chaque finalité, les données principales, la durée et le
retrait.

#### H-04 — Documentation légale non publiable

`docs/legal/privacy-policy.md` se déclare template et conserve des placeholders
pour l'entité, l'immatriculation, l'adresse, le DPO et les dates. La section
cookies renvoie à une spécification technique, pas à une politique utilisateur
complète.

Remédiation: publier une politique de confidentialité finalisée et une politique
cookies dédiée, cohérentes avec la configuration réellement déployée.

### Moyens

#### M-01 — Inventaire des traceurs absent

Aucune table opérationnelle exhaustive n'a été trouvée avec nom/clé, domaine,
partie, finalité, base d'exemption ou consentement, données, durée, fournisseur
et mécanisme de retrait.

#### M-02 — Versions génériques

`v3` décrit simultanément le schéma local, la bannière, les finalités et les
documents backend. Une version technique n'identifie pas précisément le texte
présenté ni la configuration de chaque fournisseur.

#### M-03 — Fournisseurs et finalités mélangés

Stripe, Identity et Cloudflare sont forcés à `true` comme « fournisseurs
essentiels ». Or l'exemption s'apprécie opération par opération et par finalité,
pas par marque. Tous les traceurs d'un fournisseur ne deviennent pas essentiels
du seul fait qu'une de ses fonctions l'est.

#### M-04 — GPC déconnecté du runtime traceurs

Le backend enregistre `Sec-GPC`, mais ce signal ne semble pas alimenter
directement l'état local ni empêcher les finalités optionnelles dans le runtime
frontend. Un enregistrement d'opposition sans effet technique cohérent crée un
risque de contradiction.

#### M-05 — Retrait par rechargement

La bannière recharge la page après le choix. Le runtime sait pourtant appliquer
le retrait à chaud. Le rechargement n'est pas en soi non conforme, mais masque
les défauts d'arrêt et de purge et rend leur vérification moins directe.

### Points conformes ou solides

- options désactivées par défaut;
- refus et acceptation accessibles en un clic au premier niveau;
- pas de consentement par poursuite de navigation;
- préférences accessibles après le premier choix;
- refus mémorisé comme l'acceptation;
- expiration locale après 183 jours;
- garde-fous PostHog: allowlist, pseudonymisation, nettoyage et routes
  sensibles;
- révocation disponible côté compte;
- IP tronquée dans la preuve;
- export RGPD incluant les consentements;
- GPC détecté côté API;
- documentation interne reconnaissant que replay, heatmaps, surveys,
  expérimentation et error tracking sont des finalités séparées.

## Matrice de conformité

| Exigence                              | Verdict             | Commentaire                                        |
| ------------------------------------- | ------------------- | -------------------------------------------------- |
| Consentement préalable                | Partiel             | PostHog différé; Sentry commun initialisé trop tôt |
| Acte positif clair                    | Conforme            | Clic explicite                                     |
| Refus aussi simple que l'acceptation  | Conforme            | Un clic, même niveau                               |
| Consentement libre                    | À confirmer         | Pas de cookie wall identifié dans le périmètre     |
| Consentement spécifique               | Non conforme        | Sous-finalités agrégées                            |
| Consentement éclairé                  | Non conforme        | Information trop générale et politique incomplète  |
| Retrait aussi simple                  | Partiel             | Accès permanent, mais modèle et purge à fiabiliser |
| Preuve du consentement                | Partiel             | Données utiles, historique principal écrasé        |
| Durée limitée du choix                | Conforme localement | 183 jours; backend sans expiration équivalente     |
| Absence de pré-cochage                | Conforme            | Options à `false`                                  |
| Inventaire des traceurs               | Non conforme        | Non trouvé                                         |
| Exemption documentée par opération    | Non conforme        | Fournisseurs entiers marqués essentiels            |
| Minimisation                          | Partiel             | Bons scrubbers; configuration déployée à attester  |
| Transparence destinataires/transferts | Partiel             | Sous-traitants documentés, bannière insuffisante   |
| Privacy by design/default             | Partiel             | Défaut restrictif, modèle de dérivation permissif  |

## Architecture cible recommandée

### Source de vérité

Utiliser uniquement des décisions par finalité:

```text
audience_measurement
product_improvement
session_replay
surveys_feedback
experimentation
browser_error_reporting
```

Chaque finalité porte:

- `status`: granted/denied;
- `policy_version`;
- `notice_version`;
- `configuration_version`;
- `decided_at`;
- `expires_at`;
- `scope`: device ou account;
- fournisseurs concernés.

Les catégories deviennent des groupes d'affichage. Les fournisseurs deviennent
des métadonnées et des interrupteurs optionnels uniquement si plusieurs
fournisseurs servent réellement une même finalité.

### Séparation des stockages

- projection locale minimale pour bloquer les SDK avant choix;
- journal append-only backend pour la preuve des comptes authentifiés;
- projection backend de l'état courant;
- aucune fusion silencieuse entre appareil et compte;
- refus et retrait journalisés au même niveau que l'acceptation.

### Cycle d'un SDK optionnel

```text
état inconnu/refusé
  -> SDK non importé, aucun appel fournisseur
consentement finalité
  -> import dynamique, init minimale, capture autorisée
retrait/expiration/changement de version
  -> stop, opt-out, close, purge cookies/storage, SDK bloqué
```

## Plan de remédiation priorisé

### P0 — Avant activation production des traceurs

1. Retirer l'initialisation Sentry du transport commun.
2. Remplacer le modèle dérivé par une source de vérité par finalité.
3. Rendre les interrupteurs de finalité réellement fonctionnels.
4. Corriger les migrations v1/v2 pour ne jamais élargir un ancien choix.
5. Ajouter des tests réseau/navigateur prouvant zéro requête PostHog/Sentry et
   zéro stockage fournisseur avant consentement et après retrait.
6. Désactiver replay, heatmaps, surveys et flags en production jusqu'à ce que
   leur consentement séparé et leur configuration soient validés.

### P1 — Preuve et transparence

1. Créer le journal append-only de décisions.
2. Versionner séparément notice, politique et configuration.
3. Trancher et documenter la portée appareil/compte.
4. Finaliser la politique de confidentialité.
5. Créer la politique et l'inventaire cookies publics.
6. Relier la bannière aux documents publics et identifier les responsables.

### P2 — Gouvernance continue

1. Test CI statique interdisant tout nouveau SDK ou appel de traceur hors du
   registre.
2. Test Playwright avec interception réseau pour chaque état de consentement.
3. Scan périodique des cookies, local/session storage, IndexedDB et requêtes.
4. Revue obligatoire lors d'un changement de fournisseur, finalité ou
   configuration.
5. Preuve de purge et contrôle des durées côté fournisseurs.

## Scénarios de recette obligatoires

1. Première visite sans action: aucune requête ni stockage optionnel.
2. Tout refuser: même accès fonctionnel, refus conservé 183 jours.
3. Audience seule: aucun replay, survey, heatmap, flag ou Sentry.
4. Replay seul: comportement conforme au choix de design retenu, routes
   sensibles exclues et masquage vérifié.
5. Retrait à chaud: flux stoppés et stockages purgés sans événement résiduel.
6. Expiration: retour à l'état sans choix, sans activation transitoire.
7. Changement de notice/finalité: nouvelle finalité à `false`.
8. Connexion sur second appareil: portée explicitement annoncée et résultat
   déterministe.
9. GPC actif: finalités concernées refusées techniquement.
10. SDK indisponible ou sync backend en erreur: défaut restrictif.

## Références officielles

- CNIL, « Cookies et autres traceurs: lignes directrices et recommandation »,
  29 septembre 2020:
  https://www.cnil.fr/fr/cookies-et-autres-traceurs/regles/cookies/lignes-directrices-modificatives-et-recommandation
- CNIL, FAQ cookies et traceurs, mise à jour 29 avril 2026:
  https://cnil.fr/fr/cookies-et-autres-traceurs/regles/cookies/FAQ
- CNIL, recommandations sur le consentement multi-terminaux, 16 janvier 2026:
  https://www.cnil.fr/fr/cookies-et-autres-traceurs-recommandations-finales-sur-le-consentement-multi-terminaux
- CNIL, solutions pour les outils de mesure d'audience, 4 juillet 2025:
  https://cnil.fr/fr/cookies-solutions-pour-les-outils-de-mesure-daudience
- Règlement (UE) 2016/679, notamment articles 5, 7, 12, 13 et 25:
  https://eur-lex.europa.eu/eli/reg/2016/679/oj
- Loi Informatique et Libertés, article 82:
  https://www.legifrance.gouv.fr/loda/article_lc/LEGIARTI000042388197
