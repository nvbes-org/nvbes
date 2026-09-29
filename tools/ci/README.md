# CI affected-only

Le workflow `continuous integration` planifie les validations avant d'installer
les runtimes. Les jobs sont `authorize-cache`, `scope`, `contracts`, `typescript`,
`rust`, `database`, `terraform`, `containers` et `ci-gate`.

## Sélection et repli

- PR : base explicite de la PR, head testé = commit de merge GitHub.
- Merge queue (`merge_group`) : `merge_group.base_sha` comme base explicite,
  cache autorisé en lecture (`trusted=true`, ref créée par GitHub).
- Push : dernier ancêtre ayant une CI `push` réussie sur la même branche.
- Dispatch : `base_sha` explicite ou dernier succès de la branche.
- Base absente, API indisponible, fichier racine inconnu ou échec Nx affected :
  validation complète. Un graphe Nx illisible bloque la CI.
- Les renommages sont traités comme suppression + ajout pour couvrir les deux
  propriétaires. Les références Terraform et les accès SQL sont lus dans les
  deux révisions pour couvrir les suppressions.
- Documentation seule : Node standard + Git, sans installation de dépendances.
  Les fichiers JSON et autres contrats exécutables sous `docs/` ne sont pas
  considérés comme de la documentation seule.
- CI seule : pas de calcul du graphe dans `scope`, puis tests des contrats CI.

TypeScript sélectionne chaque target disponible (`format:check`, `lint`,
`typecheck`, `check`, `test`). `build` est réservé aux projets déclarant
`metadata.ci.buildRequired: true`. Les bibliothèques `libs/ts` sont reconnues ;
une nouvelle application TS déclare `metadata.ci.runtime: typescript`.
Le lockfile est analysé par Nx, sans parseur pnpm propre au projet.

Les bases sont sélectionnées par chemins de migration/persistence, références
SQL dans le contenu des sources et dépendances partagées. Un changement Cargo
global conserve tous les tests DB par prudence. Les tests DB ne sont jamais
cacheables ; une seule instance Postgres crée les bases isolées nécessaires.

Terraform réutilise le parseur HCL déjà installé par les contrôles FinOps.
Un module local sélectionne ses consommateurs transitifs ; une entrée inconnue
ou non analysable sélectionne tous les stacks actifs. Aucune commande apply.

Un Dockerfile sélectionne son propre contrat. Les autres sources sont comparées
aux COPY/ADD locaux. Les Dockerfiles actuels copient `apps`, `libs`, `contracts`
et `vendor` en entier : une source commune peut donc sélectionner plusieurs
contrats, sans déclencher Rust pour un changement Dockerfile seul.

## Rollout et preuves

`rollout-policy.json` est livré en mode `shadow`. Docs, CI et Dockerfile seul
bénéficient immédiatement du périmètre réduit. Pour les autres changements,
les runtimes requis exécutent le candidat puis le complément du périmètre
complet. Les tests DB ne sont pas répétés. Rust compare explicitement le test
des crates candidates avec le workspace complet pour révéler les différences
de features ; un changement global ne répète pas une commande identique.

Chaque lane publie ses métriques dans les logs et le résumé GitHub. Rust publie
`CI_RUST_EVIDENCE`, les autres lanes `CI_METRIC`, et le scope `CI_SCOPE`.
Une erreur du candidat ou de la référence bloque la CI. Le gate refuse aussi
les jobs annulés, manquants, et les jobs attendus mais ignorés.

Commande opérateur, en lecture seule, après les premières exécutions :

```sh
node tools/ci/rollout-report.mjs 100
```

Le rapport `.nx/ci/rollout-report.json` compte les minutes de runner arrondies
par job, les résultats et les observations Rust. Le taux de cache Nx reste
`null` si les logs n'exposent pas de compteurs ; il n'est jamais déduit du simple
succès d'une restauration. Les identifiants de run, SHA source et numéros de PR
sont recoupés avec l'API GitHub. Des logs expirés bloquent la collecte.
Les jobs annulés avant leur démarrage n'ont pas de journal : ils sont exclus
de la lecture des logs uniquement si l'API confirme l'absence d'étapes exécutées.
Tous les journaux des jobs effectivement démarrés restent requis.

Le tableau `measurements` conserve une entrée par exécution avec son URL,
sa catégorie (`docs`, `ci`, combinaison de runtimes ou `fallback-full`), son
mode shadow, le temps de planification, le temps écoulé entre le premier et
le dernier job, et les minutes de runner arrondies séparément par job.
Comparer uniquement des catégories et modes équivalents. Les anciens logs
sans catégorie restent `unclassified`, et une mesure absente reste `null`.

L'activation nécessite au moins sept jours d'observation, vingt PR Rust
distinctes avec un candidat strictement plus petit, zéro divergence et une
revue opérateur. Conserver le rapport dans un commit signé, puis renseigner
`activationEvidence` avec `reportCommit`, `comparedPrs`, `observationDays` et
`divergences`, avant de passer `mode` à `affected`. Le validateur refuse une
activation sans ces preuves. Le rapport doit être enregistré sous
`docs/ci/affected-rollout-evidence.json` dans le commit référencé ; ses compteurs
sont recoupés avec la politique au démarrage du planificateur.
Le repli complet reste actif après activation.
Revenir à `shadow` rétablit les validations complètes des runtimes requis.

État initial mesuré le 7 septembre 2026 : cinq exécutions précédentes collectées,
quatre réussies, trente minutes de runner arrondies au total. Aucune observation
du nouveau workflow à cette date : les gains et les vingt PR restent à mesurer.
Comparer des changements de même catégorie avant de conclure sur les économies.

## Cache et FinOps

Les archives Scaleway sont restaurées uniquement depuis `trusted` et publiées
uniquement après succès de la lane sur push `main`.
Les PR internes signées peuvent lire le cache après autorisation depuis la base
de la PR ; forks et Dependabot fonctionnent sans secrets. Le cache de compilation
S3 sccache est en lecture seule sur PR, et en lecture/écriture sur push protégé.
Les contextes sans credentials utilisent le backend natif GitHub Actions de sccache,
avec un dossier de cache isolé par job et des statistiques affichées au démarrage.

TypeScript/contrats/containers : pnpm + Nx ; Rust : Cargo + sccache ; DB :
pnpm + Cargo + sccache ; Terraform : pnpm + plugins. Les namespaces Nx sont
séparés par lane, OS, architecture, version majeure Nx et version Node pour
éviter les écritures concurrentes sur la même base de cache.
Le namespace v3 inclut l'index SQLite Nx : `NX_WORKSPACE_DATA_DIRECTORY`
pointe vers `.nx/cache/workspace-data`. Sans cet index, copier les résultats
de tâches ne permet pas à Nx de les retrouver sur le runner suivant.
Le daemon est désactivé afin de fermer la base avant la publication.
La synchronisation tient compte des dates de modification : la taille seule
ne permet pas de détecter une mise à jour de l'index SQLite.
Un test d'intégration transfère le cache entre deux dossiers indépendants
et vérifie la restitution du résultat par le moteur Nx installé.

Aucun service payant, abonnement ou stockage supplémentaire n'est créé.
Le gate FinOps conserve le plafond global de 30 EUR TTC/mois et est exécuté
directement dans le job `scope` dès que le graphe est requis, précédant ainsi
toutes les exécutions de lanes. Les six lanes (`contracts`, `typescript`,
`rust`, `database`, `terraform`, `containers`) s'exécutent ensuite en parallèle
direct et simultané. Le mode `affected` est actif par défaut, avec possibilité
de sélectionner `shadow` lors d'un déclenchement manuel. Le workflow n'active
aucun dépassement payant.

## Protection de main

`ci-gate-policy.json` ajoute une politique indépendante sans supprimer les
protections existantes. Son application vérifie d'abord l'accès aux rulesets
et une exécution `push main` réussie contenant le job `ci-gate` :

```sh
node tools/ci/require-ci-gate.mjs
node tools/ci/require-ci-gate.mjs --apply
```

Au 7 septembre 2026, GitHub refuse l'accès aux rulesets avec HTTP 403 (offre
du dépôt privé). La politique est prête mais son activation distante est
bloquée ; aucun changement d'abonnement ni de visibilité n'est effectué.

## Validation locale

```sh
pnpm nx run ci-contracts:test:ci-contract --outputStyle=static
pnpm nx run ci-contracts:test:lockfile-integration --outputStyle=static
pnpm check:finops
actionlint .github/workflows/ci.yml
node tools/security/check-ci-cd-security.mjs --workflow .github/workflows/ci.yml
```
