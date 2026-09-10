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
la liaison de son contenu au SHA exige toujours la collecte CI authentifiée.
Un export dont des fichiers entiers auraient été omis doit être détecté par
le futur contrôle de complétude de campagne, pas par une comparaison naïve
avec tous les modules Rust (certains n'ont aucun compteur applicable).

## Limites à fermer avant tout candidat

1. Relier chaque ID de cas à un test réellement exécuté ; terminer le
   classement de toutes les exigences historiques et migrer les
   consommateurs Account ensemble. Les IDs actuels sont des obligations,
   pas des assertions d'exécution.
2. Produire les enveloppes depuis les runners CI, lier les artefacts à leurs
   producteurs. Les recalculs TypeScript et Rust sont intégrés ;
   l'authentification de la collecte CI, l'exhaustivité de l'instrumentation
   et des sites de mutation et la vérification du
   contenu métier des preuves de suites/opérations restent à terminer.
   La signature du paquet et la vérification d'un run ne prouvent pas à
   elles seules que GitHub a produit ces artefacts.
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
Un rapport incomplet, vide, ignoré ou sous 90 % échoue. Ce target n'est pas
encore une campagne exhaustive de tous les packages.
