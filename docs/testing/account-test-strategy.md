# Stratégie de test Account

> **Statut : harness historique à réévaluer lors de la reconstruction
> d'Account.** Account appartient au socle V1, mais les applications couvertes
> par ce document sont archivées et ne constituent pas le runtime actif. Toute
> réactivation suit la
> [direction produit](../product/nvbes-product-strategy.md) et le budget global.

## Portée et statut de preuve

Cette stratégie couvre `account-service`, `account-worker`, `account-web` et les
bibliothèques directes déclarées dans
[`account-test-manifest.json`](account-test-manifest.json). Le manifeste est le contrat
machine-readable catégorie → lane → commande → artefact. Une catégorie sans harness
réel est marquée `gap`; elle n'est jamais assimilée à un test exécuté.

Le validateur `pnpm nx run account-quality:test` bloque :

- une catégorie manquante ou dupliquée;
- une commande automatisée sans artefact;
- un cron divergent du workflow;
- une fausse déclaration d'automatisation pour fuzz, DRP, scalabilité ou
  cross-platform;
- une déclaration de scalabilité couverte sans vérification control-plane;
- la sérialisation d'un cookie de session dans `setup_data` ou un résumé k6.

## État d'automatisation réel

| Lane             | Déclenchement réel                      | Couverture actuelle                                                                     | Preuve                                              |
| ---------------- | --------------------------------------- | --------------------------------------------------------------------------------------- | --------------------------------------------------- |
| PR               | CI du dépôt sur projets affectés        | format, lint, types, unitaires, composants, contrats, migrations et sécurité ciblée     | rapports Nx/Cargo/Vitest                            |
| Scheduled daily  | cron `17 1 * * *` ou dispatch `nightly` | routes publiques Playwright Chromium/Firefox/WebKit et charge publique `load`           | `account-browser-<run-id>`, `account-load-<run-id>` |
| Scheduled weekly | cron `31 1 * * 0`                       | `volume`, `spike`, `stress`, sérialisés                                                 | `account-<profile>-<run-id>`                        |
| DAST weekly      | cron `41 2 * * 3`                       | canaries authentifiés puis ZAP web/API/backoffice staging                               | `zap-dast-<run-id>`                                 |
| Trusted hermetic | commande opérateur                      | suite Account, E2E critique Chromium, random walk public borné et charge locale         | rapports Nx/Cargo/Playwright/k6                     |
| Pre-release      | session signée                          | Alpha/Beta/UAT, usability, exploration, pentest, conformité, recovery/DRP, localisation | rapports signés                                     |

Le workflow planifié échoue si `ACCOUNT_TEST_AUTOMATION_ENABLED` n'est pas exactement
`true`. Les jobs ne peuvent donc pas produire un workflow vert en étant silencieusement
ignorés. Toutes les campagnes partagent une même clé de concurrence et la matrice
hebdomadaire utilise `max-parallel: 1`.

### Limites explicites

- Aucun harness `cargo-fuzz` ou équivalent n'est branché : `fuzz` est un gap.
- Le DAST authentifié staging est configuré, mais ses credentials statiques ne sont pas
  renouvelés pendant un scan et ZAP ne remplace ni le pentest indépendant ni une traversal
  exhaustive de chaque état SPA.
- Aucune matrice Linux arm64/macOS/Windows n'est branchée : `cross-platform` est un gap.
- Le RBAC applicatif et les scopes OAuth sont testés; l'isolation tenant/RLS Account reste
  un gate ouvert tant que le rôle runtime et les migrations de production ne l'imposent pas.
- Les pages authentifiées ne font pas encore partie de la matrice quotidienne
  cross-browser/accessibilité; l'acceptation authentifiée staging est Chromium.
- Le random walk Playwright public n'est pas un monkey test model-based des états
  login/register/MFA/OAuth avec shrinking; cette catégorie reste un gap.
- Les seuils de couverture quantitatifs ne couvrent actuellement que `http-client`.
- SMTP reste at-least-once entre acceptation provider et commit du ledger. Le Message-ID
  stable aide à réconcilier un doublon, sans garantir sa déduplication par le provider.
- DRP/failover et recovery restent des exercices humains signés; ils ne sont pas
  planifiés par GitHub Actions.
- La scalabilité est un gap explicite : le comparateur 1/2/4/8 valide les charges et
  produit seulement un verdict `blocked` tant qu'aucun adaptateur control-plane de
  confiance ne prouve les replicas réellement prêts.
- La localisation reste une session pre-release signée tant qu'une matrice de locales
  automatisée n'existe pas.
- Les rapports humains restent des campagnes opérateur, mais la promotion production
  refuse désormais tout paquet incomplet, expiré, altéré, non signé ou lié à un autre
  release. La signature Ed25519 et l'attestation control-plane de déploiement ne
  remplacent pas la revue humaine du contenu.

Ces limites sont des gates de release ouvertes, pas des succès implicites.
`pnpm check:account-release-readiness`, appelé par le gate production, échoue tant
qu'une catégorie est un `gap`, porte une `limitation`, ou appartient à un
`knownGap` déclaré `releaseBlocking`. Le registre
`releaseReadiness.blockingCategories` doit correspondre exactement à cette
union; le validator refuse tout drift. Le manifeste est l'unique source de vérité
pour le nombre et la liste courants : `pnpm check:account-release-readiness` les
recalcule et les affiche lorsqu'il bloque. La documentation ne duplique
volontairement pas cette liste afin d'éviter qu'elle dérive du gate exécutable.
Aucun waiver silencieux n'est prévu pour le socle V1.

## Fiabilité et sécurité du harness

- Les campagnes k6 acceptent `ci`, `development`, `local`, `test` ou `staging`.
- En local/CI, seules les origines HTTP loopback et
  `host.docker.internal` pour le service sont acceptées.
- En staging, HTTPS et une correspondance exacte dans
  `ACCOUNT_LOAD_ALLOWED_ORIGINS` ou `ACCOUNT_WEB_ALLOWED_ORIGINS` sont obligatoires.
- Les userinfo, ports non allowlistés, hôtes encodés, chemins, query strings, fragments,
  adresses link-local/privées et hôtes de production sont rejetés.
- Les inputs `workflow_dispatch` ne peuvent donc pas servir à charger une cible tierce
  ou une URL interne.
- k6 ne suit aucun redirect; une cible allowlistée ne peut pas transférer la charge
  vers un autre origin.
- Les comptes et cookies de charge sont synthétiques. Le workflow GitHub public
  n'accepte aucun cookie.
- `setup()` ne retourne aucune donnée. k6 sérialise sinon la valeur dans
  `K6_SUMMARY_EXPORT.setup_data`.
- Après chaque run, le résumé est parsé et rejeté s'il contient `setup_data`, un champ
  credential-bearing ou la valeur exacte du cookie.
- La métadonnée archivée est construite par allowlist de champs; l'environnement
  complet n'est jamais sérialisé.
- Un résumé absent fait échouer le runner et `if-no-files-found: error` rend les preuves
  CI obligatoires.

Les campagnes longues ne ciblent jamais la production. L'allowlist staging est une
variable de dépôt protégée et doit contenir des origines exactes séparées par des
virgules, jamais des wildcards.

## Lane hermétique et dépendance Cloud

`pnpm nx run account-quality:test-nightly` est une commande de runner de confiance, pas
un job actuellement planifié. Elle exige des runtimes locaux isolés :

- Account web et Account service;
- PostgreSQL éphémère migré depuis zéro;
- Redis et les providers de test requis;
- un sink email test isolé, privé et éphémère via `NVBES_EMAIL_TEST_CAPTURE_DIR`;
- Cloud gRPC via `NVBES_CLOUD_GRPC_ENDPOINT`;
- identifiants Beta synthétiques.

Le seed Beta du E2E critique traverse réellement la frontière Cloud gRPC. Une endpoint
locale explicite est obligatoire; l'absence du runtime Cloud échoue avant les tests au
lieu de devenir un transport error ambigu ou un skip. Les tokens email ne sont pas lus
dans le payload durable de la queue : le worker écrit le sink test avec permissions
restreintes, le helper exige ensuite un statut `sent`/`delivered` dans le ledger, puis
consomme le fichier. Le random walk public déterministe reste exécuté après les matrices.

## Profils k6 et seuils

Les durées sont contractuelles. Un override différent de la durée du profil échoue.

| Profil      |     Durée exacte | Forme                                  | Gate de débit                               |
| ----------- | ---------------: | -------------------------------------- | ------------------------------------------- |
| smoke       |            1 min | VU constant                            | minimum une itération par VU                |
| load        |           20 min | arrivée progressive et pic             | ≥ 98 % des itérations planifiées, 0 dropped |
| volume      |       max 30 min | 100 000 itérations partagées           | 100 % des itérations demandées              |
| spike       |            6 min | saut 20 → 500 it/s puis recovery       | ≥ 98 % planifiées, 0 dropped                |
| stress      |           15 min | arrivée 10 → 50 → 100 → 250 → 500 it/s | ≥ 171 990 / 191 100, dropped ≤ 19 110       |
| soak        |              4 h | arrivée constante                      | ≥ 98 % planifiées, 0 dropped                |
| scalability | 15 min par point | arrivée constante                      | ≥ 98 % de la charge configurée, 0 dropped   |

Seuil nominal :

- checks > 99,5 %;
- erreurs HTTP < 0,5 %;
- p95 < 400 ms;
- p99 < 800 ms.

Le stress autorise checks > 98 %, erreurs < 2 %, p95 < 1 500 ms et p99 < 3 000 ms
afin de mesurer une dégradation contrôlée. Load, spike, soak et scalabilité refusent tout
`dropped_iteration`; le stress autorise au plus 10 % de la charge planifiée afin de mesurer
le point de saturation sans pouvoir passer avec une majorité de travail abandonné.

Les variables k6 réservées `K6_VUS`, `K6_DURATION` et `K6_ITERATIONS` ne configurent
jamais les scénarios exportés. Les overrides internes utilisent exclusivement le
préfixe `ACCOUNT_K6_`.

## Scalabilité

Le profil de charge structure quatre points 1, 2, 4 et 8 replicas avec le même release,
la même cible et le même dataset synthétique. Le comparateur revalide schéma, digests,
durée exacte de 15 minutes, SLO et cohérence des huit fichiers, mais un nombre de
replicas déclaré par l'environnement n'est pas une preuve d'infrastructure.

La commande de verdict est :

```bash
ACCOUNT_SCALABILITY_COMPARISON_FILE=/evidence/comparison.json \
ACCOUNT_LOAD_ALLOWED_ORIGINS=https://account.staging.example \
K6_PROFILE=scalability \
K6_SUITE=account-api \
pnpm nx run account-quality:test-resilience
```

Le contrat échoue si un point manque, si release/cible/durée/suite diffèrent, si des
itérations sont dropped, si le débit atteint moins de 98 % du débit configuré, si le
taux d'erreur dépasse 0,5 % ou si l'efficacité par replica tombe sous 70 %. Même si ces
contrôles passent, il écrase tout ancien verdict vert par `verdict: "blocked"` puis sort
en erreur. Fermer ce gap exige des observations control-plane authentifiées avant et
après chaque run, liées au deployment UID, au release immuable, au digest du résumé et à
une trust root configurée.

## Compatibilité et acceptation

La lane quotidienne exécute Chromium, Firefox et WebKit sur les projets Playwright
déclarés. Cela couvre les routes publiques, le consentement et le cross-browser, pas le
cross-platform OS/architecture ni les pages authentifiées. Les tests UI publics incluent
accessibilité automatisée, cookies, navigation clavier, viewports et mouvement réduit.
Les cookies session/CSRF et l'acceptation authentifiée staging passent dans les lanes
Chromium dédiées.

Les tests Alpha, Beta, UAT, usability, exploratory, black-box, localisation, pentest,
conformité et DRP suivent
[`account-acceptance-charters.md`](account-acceptance-charters.md). Une session non
signée ou un artefact absent vaut échec. `pnpm release:gate:production` est un gate
post-déploiement : il exige le paquet d'acceptation signé lié au SHA du release, une
attestation control-plane de ce même déploiement datant de moins de 24 heures, puis les
URLs de production allowlistées pour le smoke. Le préflight refuse les DNS non publics,
les redirects et un `/health.release_id` différent du SHA attendu.

## Principes de qualité

- Aucun skip d'infrastructure, de schéma ou de provider ne devient vert.
- Aucun retry ne transforme un échec en succès.
- Les tests concurrents utilisent des namespaces indépendants.
- Les tests DB refusent les cibles non-loopback/non-test; le harnais de migration restaure
  exactement les bases et rôles cluster-wide qu'il introduit.
- Les erreurs, logs et artefacts sont redigés et ne contiennent aucune donnée
  personnelle, clé, cookie ou token.
- Un bug critique commence par un test rouge au niveau le plus bas qui le reproduit.
- Les seuils sont des planchers versionnés; leur baisse exige une décision de risque
  datée.
