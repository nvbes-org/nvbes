# Charters d'acceptation Account

> **Statut : charters historiques à réévaluer.** Conserver uniquement les
> scénarios compatibles avec Account et les équipes B2C du socle ; les parcours
> Cloud, Backoffice ou Enterprise sont hors V1.

## Regles communes

Ces validations ne sont pas remplacees par un test automatise. Chaque session produit :

- build, environnement, navigateur, plateforme et feature flags;
- profil du participant anonymise et consentement de recherche;
- donnees synthetiques utilisees;
- taches, resultat, temps, erreurs et severite;
- observations sans secret ni donnee personnelle;
- decision, owner et date cible;
- preuve signee dans le dossier de release.

Une anomalie critique de securite, perte de donnees ou blocage du parcours principal
arrete la campagne et la promotion.

## Paquet de preuve de production

La promotion production consomme un paquet versionné et signé contenant exactement les
rapports Alpha, Beta fermée, Beta ouverte, UAT, usability, exploratory, black-box,
penetration, conformité, recovery, DRP/failover et localisation. Chaque entrée déclare
`category`, `decision: "passed"`, `signedBy`, `signedAt`, `release`, `environment:
"staging"`, un chemin relatif et le SHA-256 du rapport. Le pentest déclare aussi
`independent: true`. Tous les timestamps utilisent la forme UTC canonique
`YYYY-MM-DDTHH:mm:ss.sssZ`.

Le JSON racine déclare `schemaVersion: 1`, le SHA immuable du release, `environment:
"staging"`, `issuedAt`, `expiresAt`, `signerKeyId`, `reports` et `deployment`.
`deployment` référence le chemin, le SHA-256, la signature détachée et le key ID d'une
seconde attestation. Celle-ci est signée par une clé distincte détenue par le control
plane et destinée à `nvbes-account-release-gate`. Elle lie le même release aux trois
composants exacts `account-service`, `account-worker` et `account-web`, avec pour chacun
deployment UID attendu, digest d'image, replicas désirés/prêts et date d'observation.
La validité maximale du paquet est de 31 jours et chaque observation de déploiement
doit dater de moins de 24 heures. Chaque décision de rapport possède un `signedAt`
antérieur à `issuedAt` et sa propre fraîcheur maximale : 31 jours pour le pentest,
14 jours pour les deux preuves Beta, et 7 jours pour les neuf autres catégories. Une
décision plus ancienne, liée à un autre release ou environnement, échoue même si le
paquet racine est encore valide.

Le mécanisme actuel signe cryptographiquement le manifeste racine et donc les
attributions, décisions et digests qu'il contient; il ne signe pas chaque rapport avec
une clé indépendante. `signedBy` et `independent: true` sont des déclarations couvertes
par la clé du release board, pas une preuve cryptographique de séparation des tâches.
Alpha, Beta, UAT, pentest, conformité et les autres validations humaines restent donc
release-blocking jusqu'à l'ajout de signatures détachées par rapport et d'allowlists de
key IDs/organisations propres à chaque catégorie, avec une trust root pentest distincte.

Le fichier JSON exact est signé hors dépôt avec une clé Ed25519, via
`sign-acceptance-evidence.mjs` et la commande `pnpm sign:account-acceptance-evidence`
(voir §7 de `iso-29119/03-documentation.md`) :

```bash
pnpm sign:account-acceptance-evidence \
  --evidence-root <dossier-paquet> \
  --private-key account-acceptance-private.pem \
  --release <SHA-immuable> \
  --signer-key-id <key-id-release-board> \
  --deployment-key-id <control-plane-key-id> \
  --signed-by "Alpha Owner@acceptance-alpha" \
  --signed-by "Beta Owner@acceptance-beta" \
  --public-key account-acceptance-public.pem
```

La clé privée ne doit jamais entrer dans le dépôt ou un runner de vérification. Les
clés publiques, key IDs, fingerprints, UIDs, cibles et allowlists proviennent
uniquement des GitHub Environments protégés décrits ci-dessous. Une invocation locale
de `scripts/release-gate.sh production` ne possède ni l'approbation Environment, ni la
provenance Actions, ni la sélection immuable des artefacts : elle peut servir au
diagnostic, mais n'est jamais une promotion. Le seul déclencheur de production est
`.github/workflows/account-release.yml`.

Les deux key IDs, les deux empreintes et les UIDs attendus sont des variables protégées
du pipeline; le signataire des tests ne peut donc ni substituer la trust root du control
plane, ni attester un autre composant. Le validateur refuse les symlinks, sorties de
répertoire, doublons, catégories manquantes, digests divergents, signature ou trust
root invalide, paquet expiré, mauvais release, composant inattendu, replica non prêt et
attestation absente ou périmée. Le smoke production reste obligatoire après cette
vérification. Avant le smoke, les deux URLs doivent être des origines HTTPS exactes de
l'allowlist production protégée, résoudre uniquement vers des adresses publiques et ne
pas rediriger; `/health` doit exposer exactement `NVBES_RELEASE_SHA`.

Avant même de vérifier ces signatures, `pnpm check:account-release-readiness` calcule
l'union triée de chaque contrat `execution: "gap"`, de chaque `limitation` non vide et
des catégories de chaque `knownGaps` avec `releaseBlocking: true`. Cette union doit être
strictement identique à `releaseReadiness.blockingCategories`; tout registre périmé
échoue. Dans l'état actuel, 39 catégories sont bloquantes : `acceptance-alpha`,
`acceptance-beta`, `access-control`, `accessibility`, `beta`, `black-box`,
`compatibility`, `compliance`, `cookies`, `cross-browser`, `cross-platform`,
`data-migration`, `drp-failover`, `end-to-end`, `end-to-end-integration`, `endurance`,
`exploratory`, `fuzz`, `global-regression`, `grey-box`, `load`, `localization`,
`monkey`, `penetration`, `performance`, `recovery`, `reliability`, `sanity`,
`scalability`, `security`, `smoke`, `spike`, `stress`, `system-integration`, `ui`,
`usability`, `user-acceptance`, `volume` et `white-box`. S'y trouvent explicitement
l'absence de signatures indépendantes par rapport, d'importer confidentiel pinné et de
manifeste d'artefacts automatisés exact-SHA.

### Frontière de confiance release GitHub

Les trois workflows n'acceptent aucun input et checkout le `github.ref` sélectionné.
`verify-account-release-ref.mjs` exige un tag léger
`account-rc-<SHA-commit-complet>` et vérifie en ligne que la ref GitHub courante pointe
directement vers le même `github.sha`; branche, tag annoté, tag déplacé ou SHA partiel
échoue. Un repository ruleset doit réserver la création de `account-rc-*` aux release
managers et interdire update, force-update et suppression. Les Environments
`account-acceptance-source`, `account-acceptance` et `production-account` autorisent
uniquement le pattern de tag `account-rc-*`, imposent des reviewers indépendants et
interdisent l'auto-approbation. Ils ne sont pas limités à `main` : les campagnes
humaines continuent sur le RC immuable même si `main` avance.

La chaîne d'import n'est volontairement pas active :

1. `.github/workflows/account-acceptance-source.yml` valide le RC et le scanner CI/CD,
   puis exécute inconditionnellement `exit 1`. Il ne référence aucun secret, ne lit
   aucun dépôt d'évidence et ne publie aucun artefact.
2. `.github/workflows/account-acceptance-ingest.yml` conserve le contrat du
   consommateur exact-SHA, mais exige un run source réussi du même workflow et du même
   tag. Le producteur fail-closed rend cette condition impossible; aucun run ID ne doit
   être configuré et aucun rapport brut ne doit être uploadé dans les artefacts Actions
   du dépôt applicatif.
3. `.github/workflows/account-release.yml` reste l'unique déclencheur production. Il
   vérifie le RC puis attend un run ingest exact. En l'absence d'importer sûr, cet
   artefact n'existe pas; de plus, le readiness gate bloque les 39 catégories.

Fermer ce blocage exige un importer externe pinné, non modifiable par le code du RC,
qui obtient un credential éphémère GitHub App ou OIDC limité en lecture au dépôt
d'évidence privé ID-pinné. Les rapports restent dans un store ACL ou chiffré; les
artefacts Actions du dépôt applicatif ne contiennent que reçus redacted, digests et
attestations exact-SHA. L'importer doit borner l'archive, refuser symlinks/doublons/path
traversal, vérifier signatures et signer une attestation de résultat. Il doit aussi
éliminer le TOCTOU DNS en liant la connexion TLS à une adresse publique déjà validée,
pas seulement résoudre puis refaire une connexion par hostname. Ces exigences sont un
`knownGap` release-blocking, pas une procédure opérateur disponible.

Configuration active par Environment :

- `account-acceptance-source` ne possède actuellement aucun secret ni variable
  d'accès aux rapports;
- `account-acceptance` réserve `ACCOUNT_ACCEPTANCE_SOURCE_RUN_ID`, les variables
  `ACCOUNT_ACCEPTANCE_TRUSTED_KEY_ID`,
  `ACCOUNT_ACCEPTANCE_TRUSTED_PUBLIC_KEY_SHA256`,
  `ACCOUNT_DEPLOYMENT_TRUSTED_KEY_ID`,
  `ACCOUNT_DEPLOYMENT_TRUSTED_PUBLIC_KEY_SHA256`,
  `ACCOUNT_DEPLOYMENT_EXPECTED_COMPONENT_UIDS` et les secrets
  `ACCOUNT_ACCEPTANCE_TRUSTED_PUBLIC_KEY_PEM`,
  `ACCOUNT_DEPLOYMENT_TRUSTED_PUBLIC_KEY_PEM`, mais aucun run ID n'est valide tant que
  le producteur reste bloqué;
- `production-account` possède `ACCOUNT_ACCEPTANCE_EVIDENCE_RUN_ID`, les cinq
  variables de trust, les deux secrets de clés publiques, le secret
  `NVBES_STAGING_ACCOUNT_PASSWORD`, les variables
  `NVBES_STAGING_ACCOUNT_EMAIL`, `NVBES_STAGING_WEB_BASE_URL`,
  `NVBES_STAGING_API_BASE_URL`, `NVBES_STAGING_ALLOWED_WEB_ORIGINS`,
  `NVBES_STAGING_ALLOWED_API_ORIGINS`, `ACCOUNT_PRODUCTION_DENIED_ORIGINS`,
  `NVBES_PRODUCTION_WEB_BASE_URL`, `NVBES_PRODUCTION_API_BASE_URL`,
  `NVBES_PRODUCTION_ALLOWED_ORIGINS`, `NVBES_FAPI_HIGH_ASSURANCE_ENABLED` et
  `NVBES_FAPI_CONFORMANCE_EVIDENCE_SHA256`.

Pour reproduire le blocage attendu, un release manager crée et pousse un tag léger
`account-rc-<SHA>` sur le commit exact, puis dispatch
`account-acceptance-source.yml` en sélectionnant ce tag. Le run doit valider la ref et
terminer sur `Block candidate-controlled evidence import`. Il ne faut ni configurer un
run ID ingest, ni dispatcher la production. Il n'existe aucune séquence exécutable de
promotion tant que l'importer externe, la confidentialité, les signatures par rapport
et le manifeste de preuves automatisées exact-SHA ne sont pas implémentés.

Si le profil FAPI est activé, le rapport `compliance` du paquet est le JSON officiel de
conformance FAPI final à l'emplacement `fapi-conformance.json`; son digest protégé doit
égaler `NVBES_FAPI_CONFORMANCE_EVIDENCE_SHA256`. Le gate d'opérations consomme aussi,
au SHA checkouté, `docs/compliance/external-security-assurance.json` : activation
production approuvée, pentest indépendant et retest valides, zéro finding critique ou
high, références `vault://`, et cinq scopes d'authentification privilégiée
phishing-resistant passés. Un asset signé ne remplace aucune de ces preuves.

## Alpha interne

### Entree

- gates PR et main verts;
- migrations testees sur base fraiche et upgrade N-1;
- zero vulnerabilite critique ouverte;
- rollback par restauration/forward-fix execute et signe en pre-release;
- observabilite et support operationnels.

### Participants

Engineering, Security, Product, Support et SRE, avec comptes utilisateur, admin,
security-admin et service account.

### Charters

1. Inscription, region, consentement, verification email et premier login.
2. Password, passkey, TOTP, recovery code, step-up et perte de facteur.
3. Deux comptes ouverts : switch, caches, cookies, CSRF et logout independants.
4. Sessions/appareils : trust, risque, revoke, password change et theft detection.
5. OAuth : approve/deny, PKCE/PAR, scopes, refresh, revoke et client lifecycle.
6. Privacy : consentements, GPC, export, suppression et retention.
7. Worker : panne SMTP/Redis/PostgreSQL/S3/gRPC, reprise, détection et réconciliation
   des doublons éventuels.
8. Accessibilite clavier/lecteur d'ecran et zoom 200 %.

### Sortie

- 100 % des charters executes;
- zero P0/P1 ouvert;
- P2 acceptes explicitement avec owner/date;
- smoke, logs, alertes et runbooks verifies.

## Beta fermee

### Cohortes

- particuliers avec un et plusieurs comptes;
- equipe avec owner/admin/member;
- utilisateurs passkey et MFA de secours;
- navigateurs et appareils de la matrice supportee;
- besoins d'accessibilite et langues supportees.

### Mesures

- taux de completion inscription/login/MFA/reset;
- temps et abandons par etape;
- recours recovery/support;
- comprehension du compte actif et des permissions;
- erreurs API et Web Vitals;
- jobs perdus/dupliques, backlog et temps de livraison email;
- incidents privacy/security.

### Sortie

- objectifs produit signes avant le debut de la cohorte;
- zero incident de severite critique;
- taux de succes des parcours critiques >= 99 % hors erreur utilisateur;
- accessibilite critique/serieuse a zero;
- feedback classe, decide et tracable.

## Beta ouverte

La Beta ouverte reutilise les criteres de Beta fermee, avec en plus :

- capacite et alertes validees au volume attendu x2;
- support, statut public et communication incident prets;
- rate limits et anti-abus testes;
- surveillance des cohortes sans collecte superflue;
- kill switch et rollback verifies.

## User acceptance testing

Les representants Product, Security, Support et Operations signent les parcours
Given/When/Then suivants :

- un utilisateur legitime peut accomplir chaque parcours critique;
- un acteur sans role/scope ne peut ni lire ni modifier la ressource;
- un changement sensible exige le niveau de step-up prevu;
- une panne partielle produit un message recuperable et aucun etat incoherent;
- export, suppression, consentement et retention correspondent aux politiques publiees;
- les operateurs disposent des traces et actions de reprise attendues.

Le verdict est `accepted`, `accepted-with-dated-risk` ou `rejected`. Une acceptation avec
risque n'est jamais autorisee pour une violation d'acces, une perte de donnees ou un
secret expose.

## Usability testing

### Profils

- utilisateur non technique;
- utilisateur multi-compte;
- administrateur d'equipe;
- utilisateur passkey/MFA;
- utilisateur clavier ou technologie d'assistance.

### Taches

1. Creer et verifier un compte.
2. Se connecter puis identifier le compte actif.
3. Ajouter une passkey et conserver les recovery codes.
4. Revoquer une session inconnue.
5. Changer email et mot de passe.
6. Comprendre et modifier les consentements.
7. Exporter puis demander la suppression des donnees.
8. Autoriser puis revoquer une application OAuth.

Mesurer completion, temps, erreurs non recuperees, besoin d'aide, confiance et
comprehension. Aucun verbatim n'est conserve sans consentement explicite.

## Exploratory testing

Charters timeboxes de 60 a 90 minutes :

- transitions arriere/avant/reload pendant login, MFA, OAuth et reset;
- multi-onglets et deux comptes actifs;
- horloge decalee, expiration et rotation pendant une action;
- offline/online, latence, 429, 5xx et reponse malformee;
- Unicode, RTL, noms longs, tailles et volumes extremes;
- cookies bloques, stockage plein et service worker obsolete;
- concurrence entre revoke, refresh, password change et logout;
- permissions modifiees pendant une page ouverte.

Chaque bug reproductible devient un test automatise lorsqu'un oracle stable existe.

## Black-box validation

La session black-box utilise uniquement le domaine deploye, la documentation publique
et des comptes synthetiques. Aucun acces DB, Redis, trace interne ou seed prive n'est
autorise pendant l'execution. Le rapport distingue clairement observations externes et
diagnostic interne realise apres la session.

## Localization

La matrice pre-release couvre chaque locale publiee, fallback, formats de date/nombre,
pluriels, textes longs, Unicode, RTL, zoom 200 % et technologies d'assistance. Tant
qu'une matrice automatisee n'est pas branchee, cette session signee reste obligatoire
et la categorie n'est pas annoncee comme automatisee.

## Monkey et model-based

Le test Playwright actuel est seulement un random walk déterministe et borné des routes
publiques. Il ne satisfait pas ce charter model-based, qui reste un gate ouvert.

Les actions aleatoires restent bornees aux comptes synthetiques et a staging :

- modele d'etat explicite pour login/register/MFA/OAuth;
- seed enregistree pour reproduction;
- invariants permanents : aucune elevation, aucune fuite inter-compte, aucun secret,
  aucune page blanche et aucun effet externe duplique;
- budget et nombre d'actions archives;
- shrink du scenario avant creation du bug.

## Penetration et conformite

Le pentest est realise par une partie independante avec autorisation ecrite, plage IP,
fenetre, cibles, donnees et contact incident. Il couvre API, web, OAuth/OIDC, sessions,
RBAC/RLS, business logic, SSRF, injection, upload, supply chain et cloud boundaries.

La conformite mappe ASVS, OWASP API, OAuth/OIDC/FAPI, RGPD et politiques internes aux
tests et preuves versionnees. Une preuve expiree ou non reproductible vaut echec.

## Recovery et DRP

Le depot ne contient pas de migrations SQL descendantes versionnees. Le gate automatise
des migrations prouve la creation fraiche et l'upgrade N-1, mais ne revendique donc
aucun rollback. Le retour arriere PostgreSQL repose sur une sauvegarde/PITR testee,
suivie d'un forward-fix si necessaire; sa preuve reste une campagne pre-release signee.
Le harnais snapshotte aussi tous les roles du cluster : `nvbes_system` est supprime
uniquement si la migration testee l'a cree, et un role preexistant est toujours preserve.

Les exercices pre-release couvrent :

1. restauration PostgreSQL point-in-time et validation fonctionnelle;
2. perte Redis avec re-authentification controlee et queues reconciliees;
3. indisponibilite region/provider puis failover;
4. replay outbox/DLQ sans doublon;
5. restauration des objets/audit et verification d'integrite;
6. perte de secret/KMS, rotation et revocation;
7. retour nominal et reconciliation complete.

Pour chaque exercice : RPO, RTO, donnees perdues, effets dupliques, alertes, decisions
humaines et ecarts au runbook sont mesures puis signes.

## Scalabilite

La campagne visée déploie exactement le même release avec 1, 2, 4 et 8 replicas. Chaque
point utilise le même `ACCOUNT_K6_DATASET_ID` synthétique, produit un résumé k6 et sa
metadata, puis un `comparison.json` les référence. La commande suivante vérifie la
structure et les SLO, mais termine volontairement en échec :

```bash
ACCOUNT_SCALABILITY_COMPARISON_FILE=/evidence/comparison.json \
ACCOUNT_LOAD_ALLOWED_ORIGINS=https://account.staging.example \
K6_PROFILE=scalability \
K6_SUITE=account-api \
pnpm nx run account-quality:test-resilience
```

Le dépôt ne possède pas encore d'adaptateur authentifié vers le control plane ni de
trust root pour attester les replicas prêts. Le checker écrit donc uniquement un
`scalability-verdict.json` avec `verdict: "blocked"` et refuse la promotion, même si les
résumés sont conformes. Quatre tags de replica, des variables d'environnement ou un
JSON non signé ne constituent jamais une preuve. Ce charter reste un gap jusqu'à ce
que les observations avant/après soient liées au deployment UID, au release immuable
et au digest de chaque résumé par une attestation vérifiable.

## Prerequis du E2E critique

Le seed Beta traverse la frontiere Cloud gRPC. Le runner doit fournir une endpoint
locale explicite via `NVBES_CLOUD_GRPC_ENDPOINT` et demarrer le runtime Cloud
correspondant. Un runtime absent ou non local arrete la lane avant Playwright. Le worker
utilise `NVBES_EMAIL_PROVIDER=test-capture` et un repertoire absolu, prive et ephemere
`NVBES_EMAIL_TEST_CAPTURE_DIR`; le helper ne retourne un token qu'apres la preuve
`sent`/`delivered` du ledger et supprime ensuite la capture.

Le transport SMTP de production reste at-least-once : une acceptation provider suivie
d'un echec de commit peut produire un second envoi avec le meme Message-ID. Les tests
automatises prouvent l'identite stable et la reconciliation locale, pas une garantie
exactly-once du relais. Toute campagne qui exige zero doublon doit donc verifier la
deduplication du provider choisi ou bloquer la release.
