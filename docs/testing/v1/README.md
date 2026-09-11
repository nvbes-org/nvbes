# Dispositif de test V1 — chantier ouvert

Le manifeste racine `manifest.json` référence les six domaines actifs. Il ne
déclare pas la V1 terminée. Les écarts restent ouverts tant que les campagnes
du candidat final ne prouvent pas leur clôture. Aucune certification externe
n'est revendiquée.

## Décision commune

```sh
pnpm exec nx run test-summary:catalogue
pnpm exec nx run test-summary:test
pnpm exec nx run test-summary:release
```

La dernière commande produit `.temp/test-summary/v1.json` et `v1.md`, puis
échoue sans preuve valide. `scripts/release-gate.sh` délègue à ce même
validateur ; le rapport Account historique est seulement diagnostique et
ne peut plus prononcer GO.

Les minima sont 90 % par unité de production pour les lignes, les branches
et les mutations. Les seuils historiques supérieurs sont conservés. La
fermeture transitive des dépendances Cargo inclut les dépendances locales
de production, y compris optionnelles, mais pas les dépendances de test.
Billing possède un manifeste Cargo autonome. Les packages TypeScript
déclaratifs/générés explicitement exclus sont contrôlés pour détecter tout
code runtime manuscrit ajouté.

Les anciens fichiers de seuils Rust contiennent encore des baselines et
exclusions historiques : **leurs gates seuls ne prouvent pas les objectifs
V1**. Le catalogue V1 réinclut les dépendances de production et impose le
plancher de 90 %, indépendamment de ces exclusions. Leur migration et la
campagne exhaustive restent à terminer.

## Paquet de preuves versionné

Le validateur attend un checkout propre, `NVBES_RELEASE_SHA` égal au SHA
complet de HEAD, et les variables suivantes :

- `NVBES_V1_EVIDENCE_FILE` : paquet JSON schemaVersion 1 ;
- `NVBES_V1_SIGNATURE_FILE` : signature Ed25519 binaire du JSON exact ;
- `NVBES_V1_PUBLIC_KEY_FILE` : clé publique PEM ;
- `NVBES_V1_PUBLIC_KEY_SHA256` : empreinte de confiance fournie séparément.

Ne jamais committer la clé privée ou les rapports confidentiels. Le paquet
et ses artefacts restent dans un stockage distinct. Le mécanisme de
signature existant peut signer les octets du paquet ; ne pas inventer un
second opérateur ou une revue indépendante.

Le paquet contient `sha`, `manifestDigest`, `results`, `measurements`,
`artifacts` et `operations`. Le digest du manifeste est fourni par
`test-summary:catalogue`. Chaque résultat contient :

- `suite`, `domain`, `sha`, `environment` (`isolated` ou `staging`) ;
- `status` : `passed`, `failed`, `blocked`, `not-run`, `not-applicable` ;
- `startedAt`, `completedAt` : dates UTC ISO canoniques ;
- `attempt: 1`, `tools` avec versions non vides, `cases` avec IDs et statuts ;
- `artifacts` : chemins relatifs ; `producer` : CI ou opérateur.

Chaque artefact du paquet a `path` et `sha256`. Les chemins sortant du
répertoire, y compris via un lien symbolique, sont refusés. Une preuve
absente, altérée, expirée (24 h maximum) ou pour un autre SHA bloque GO.
Un résultat automatisé référence un run GitHub du dépôt et d'un workflow
autorisé, terminé avec succès au premier essai pour le SHA candidat.
Un résultat manuel identifie `signedBy` et `publicKeyDigest` ; la signature
du paquet engage cet opérateur solo, pas un tiers indépendant.

Pour chaque fichier automatisé, `artifacts[].ci` contient `artifactId`
(identifiant numérique GitHub) et `member` (chemin exact dans son ZIP).
Chaque mesure porte aussi un `producer` GitHub Actions, au même format que
les résultats de suites. Le validateur interroge GitHub avec `gh api`,
contrôle le run, le SHA, les identifiants du dépôt et l'expiration, télécharge
le ZIP, vérifie son digest fourni par GitHub puis compare les octets du membre
aux octets locaux signés. Un artefact absent, supprimé, expiré ou sans digest
entraîne NO-GO ; aucune confiance de remplacement dans la signature locale.
Le contrat API utilisé est documenté par [GitHub](https://docs.github.com/en/rest/actions/artifacts).

La vérification nécessite `gh` authentifié avec lecture des Actions et
`unzip` disponible (les tests de ce lecteur utilisent aussi `zip`). Les
archives ne sont jamais extraites : un membre sans glob est lu sur stdout.
Bornes : 16 Mio par archive/membre, 64 Mio téléchargés et 64 Mio de fichiers
locaux, 32 archives/runs, 256 fichiers locaux ; 30 s par requête GitHub et
10 s par lecture ZIP. Les fichiers temporaires privés sont nettoyés.
Les preuves opérateur n'ont pas de référence `ci` et restent dans leur
stockage confidentiel ; elles ne peuvent pas remplacer les mesures CI.

## Reçus produits par les suites CI

`test-summary:receipt` exécute lui-même la cible Nx déclarée dans le manifeste
et écrit `.temp/v1-receipts/<suite>.json`. Le reçu contient le SHA, les dates,
les versions Node/pnpm/Nx, le domaine, la suite, le cas, le statut, la cible,
une empreinte de la commande déterministe et le producteur GitHub. Les
arguments de commande ne sont pas sérialisés, afin de ne pas recopier de
secrets éventuels. Une sortie non nulle, un signal ou une erreur de lancement
produit un statut `failed` et fait échouer l'étape.

Le producteur refuse les PR, forks, reruns, workflows inconnus et contextes
GitHub incomplets. Il accepte seulement les suites automatisées avec une cible
Nx et un unique cas `<suite>.complete-run`. Une suite métier multi-cas ne peut
pas devenir `passed` à partir du seul code retour de sa cible : elle devra
fournir un résultat vérifiable pour chaque cas avant son intégration.

Le workflow manuel V1 produit les reçus des seize suites automatisées mono-cas
(unit, check et contrôles workspace) en rejouant chaque cible Nx complète. Les
reçus sont publiés séparément pendant sept jours et l'assembleur les lie à leur
artefact GitHub exact. Les suites métier multi-cas restent volontairement
exclues de ce producteur générique : elles exigent un rapport par ID de cas.

Le workflow manuel dédié `v1-testing.yml` mesure les neuf packages TypeScript
applicables dans des lanes isolées, exécutées une à une, avec une concurrence
distincte de la CI de PR et une limite de 40 minutes par lane :
Vitest/V8 produit les compteurs de lignes et branches, puis Stryker produit les
mutants avec les timeouts comptés comme non détectés. Le reçu et les deux
rapports bruts sont publiés ensemble pendant sept jours sous le nom
`v1-measurement-typescript-<package>-<sha>`. L'assembleur recalcule les trois
scores depuis ces membres GitHub exacts avant de les ajouter au brouillon.
La mutation s'exécute même après un échec de couverture, sans convertir cet
échec en succès. Les rapports disponibles sont aussi conservés sept jours sous
`v1-diagnostic-typescript-<package>-<sha>` ; ces diagnostics ne sont pas des
reçus acceptés par l'assembleur. La matrice exhaustive ne prouve pas que ses
seuils sont atteints.

Une lane Rust protégée inventorie les crates depuis le catalogue V1, produit un
export LLVM fusionné avec les branches et exécute cargo-mutants 27.1.0 sur les
23 unités. Le producteur recalcule lignes, branches et mutations pour chaque
crate, refuse immédiatement toute valeur sous son seuil applicable, puis
publie un manifeste et les deux rapports bruts dans un unique artefact borné.
L'assembleur revalide le SHA, les versions, les dates, le producteur et tous les
scores avant d'ajouter ces 23 mesures au brouillon.

Après un run manuel terminé, l'opérateur peut construire un brouillon local :

```sh
pnpm exec nx run test-summary:assemble-receipts --runId=<trusted-run-id>
```

La commande écrit `.temp/v1-drafts/<run-id>/draft.json` et les reçus exacts
téléchargés. Elle accepte uniquement un premier essai réussi d'un workflow
autorisé, déclenché par `push` ou `workflow_dispatch` dans le dépôt canonique,
et dont la liste complète contient au plus 100 artefacts. Les artefacts nommés
comme des reçus V1 sont vérifiés intégralement ; un nom, un reçu ou une suite
invalide bloque tout l'assemblage. Les autres artefacts sont ignorés.

Ce résultat porte `incomplete: true`, ne contient ni mesures ni opérations et
n'est pas un paquet de release. Le validateur refuse explicitement un tel
brouillon, même signé : l'opérateur doit d'abord compléter toutes les suites,
les mesures et les preuves manuelles exigées, puis construire le paquet final
selon le schéma commun. Un réassemblage vers le même répertoire est refusé afin
de ne pas écraser silencieusement une collecte existante.

Les mesures identifient `unit`, `sha`, `completedAt`, `artifacts`, `lines`,
`branches`, `mutation`. `operations` contient `rpoHours`, `rtoHours`,
`targetMonthlyEurTtc`, `maximumMonthlyEurTtc` (limites 24, 8, 20, 30).
Les fixtures des tests du validateur ne sont jamais des preuves du projet.

Pour TypeScript, chaque mesure doit aussi fournir `reports` avec les clés
`istanbul-summary` et `stryker`, pointant vers deux artefacts JSON signés et
référencés dans `artifacts`. Le catalogue inventorie les sources `src/**/*.ts`
et `src/**/*.tsx`, hors tests, déclarations `.d` et fichiers `.gen`, avec
leur empreinte et la présence de code émis. Les rapports doivent inclure
chaque source runtime ; aucun fichier étranger ou dupliqué n'est accepté.
Le texte source embarqué par Stryker doit correspondre exactement au candidat.
Les types sans code émis peuvent être absents, ou avoir des compteurs nuls.
Les barrels constitués uniquement de déclarations de réexport après
transpilation sont également sans compteur obligatoire : leur inventaire et
leur empreinte restent contrôlés. Un import à effet de bord ou une expression
exécutable ne bénéficie pas de cette classification.

Les scores déclarés doivent égaler les scores recalculés, sans arrondi :
couverture depuis les compteurs entiers par fichier (pas `pct` ni le total
annoncé), mutation depuis les statuts individuels. Un timeout reste non
détecté. Les compteurs ignorés, incomplets ou sans métrique applicable
bloquent la validation ; leur éventuelle non-applicabilité nécessite encore
une décision contrôlée, jamais une conversion automatique en 100 %.

Pour Rust, `reports` doit référencer `llvm-lines`, `llvm-branches` et
`cargo-mutants`. Les exports LLVM fusionnés 3.0.1/3.1.0 sont lus par fichier
et attribués aux crates du catalogue Cargo, y compris Billing autonome.
L'inventaire comprend `src/**/*.rs` et les fichiers Rust à la racine de la
crate (dont `build.rs`). Les fichiers `.tests.rs` et `.test_support.rs` ne
contribuent pas aux scores. Les chemins inconnus dans une crate, doublons,
compteurs invalides et branches absentes bloquent la validation. Les régions
ne remplacent jamais les branches ; des compteurs nuls ne valent pas 100 %.

Le lecteur cargo-mutants 27.1.0 exige une baseline réussie, des dates de
début/fin, autant de résultats que de mutants annoncés et des phases
Build/Test cohérentes avec chaque statut. Les mutations identiques sont
refusées. Le score est `caught / (caught + missed + timeout)` ; seuls les
builds non viables sortent du dénominateur. Les trois scores déclarés doivent
égaler les valeurs recalculées sans arrondi.

Ces contrôles ne prouvent pas encore l'exhaustivité de l'instrumentation
Rust ni celle des sites de mutation. LLVM ne contient pas le texte source :
sa provenance est liée au run candidat par la comparaison avec l'artefact
GitHub, mais le producteur doit encore garantir ce qu'il a instrumenté.
Un export dont des fichiers entiers auraient été omis doit être détecté par
le futur contrôle de complétude de campagne, pas par une comparaison naïve
avec tous les modules Rust (certains n'ont aucun compteur applicable).

## Limites à fermer avant tout candidat

1. Relier chaque ID de cas à un test réellement exécuté ; terminer le
   classement de toutes les exigences historiques et migrer les
   consommateurs Account ensemble. Les IDs actuels sont des obligations,
   pas des assertions d'exécution.
2. Étendre les reçus CI aux suites métier multi-cas. Les seize suites mono-cas,
   les mesures TypeScript et Rust, leurs recalculs et la comparaison avec les
   artefacts GitHub sont intégrés et testés sur fixtures ; aucun paquet candidat
   réel complet n'a encore été vérifié. L'exhaustivité de l'instrumentation et
   des sites de mutation, ainsi que la vérification du contenu métier des
   preuves de suites et d'opérations, restent à terminer. Publier un fichier
   dans un run réussi ne prouve pas à lui seul que les cas annoncés ont été
   exécutés.
3. Étalonner nightly (épinglé à `nightly-2026-09-09`) ; étendre les mesures Rust/TypeScript et
   atteindre effectivement 90 % partout. `llvm-cov --branch` mesure les
   branches ; ce n'est pas une preuve MC/DC ni une couverture indépendante
   des conditions. Les timeouts de mutation comptent comme non détectés.
4. Finaliser le runner YAML : manifeste V1, schémas complets de paramètres,
   assertions métier et interruption avec nettoyage. Le runner sait déjà
   arrêter les dépendances, appliquer les deadlines, propager les variables
   et nettoyer ses ressources après échec/timeout. Cela ne prouve pas encore
   le nettoyage après tous les signaux système.
5. Exécuter les scénarios réels des six domaines et le parcours intégré,
   isolation Account avec rôle PostgreSQL runtime, Email at-least-once,
   fuzzing, états, charge et interfaces navigateur effectivement livrées.
6. Exécuter migration zéro/N−1, restauration, contrôles après restauration,
   exercices opérateur et FinOps avec pièces justificatives réelles.
7. Livrer la campagne complète protégée, bornée et manuelle avant release,
   puis rejouer sur le commit final. Ne pas fusionner comme « V1 terminée »
   un simple changement d'outillage.

Les six registres de domaine maintiennent ces travaux bloquants avec un
responsable et une date de revue. Public signup reste fermé, Billing en
sandbox et Trust/Risk en shadow mode.

## Mutation TypeScript

```sh
NVBES_MUTATION_PACKAGE=http-client pnpm exec nx run test-summary:mutation:typescript
```

Stryker est épinglé, utilise le runner de commandes avec Vite+ et un seul
worker. Le budget est de 30 minutes par package. Le rapport JSON est évalué
séparément, car Stryker compte normalement les timeouts comme détectés.
Un rapport incomplet, vide, ignoré ou sous le maximum de 90 % et du seuil
historique échoue. Couverture, mutation et reçu partagent le même validateur
des sources et des compteurs ; la simple production d'un fichier ne suffit pas.

L'instrumenter Stryker 10.0.0 est patché via pnpm : les assertions TypeScript
(`as const`, `as boolean`) ne doivent pas masquer leurs expressions exécutables.
Un étalonnage teste les mutations de littéraux et de booléens avant chaque
campagne et vérifie que les types seuls restent exclus.

```sh
pnpm exec nx run test-summary:campaign:typescript
pnpm exec nx run test-summary:campaign:typescript --coverage-only
```

La campagne locale parcourt les neuf unités sans abandonner les suivantes à
la première erreur, avec 10 minutes pour la couverture et 30 pour la mutation
de chacune. Elle conserve SHA, état du checkout, inventaire et statuts dans
`.temp/typescript-campaign/<date>/summary.json`. Le mode couverture seule
laisse les mutations `not-run` et échoue donc à la clôture de campagne. Ces
rapports locaux restent diagnostiques (`NO-GO`), même si tous les seuils passent :
ils ne remplacent ni la provenance CI au SHA final ni les preuves opérateur.
