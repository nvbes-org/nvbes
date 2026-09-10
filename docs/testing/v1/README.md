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

## Limites à fermer avant tout candidat

1. Relier chaque ID de cas à un test réellement exécuté ; terminer le
   classement de toutes les exigences historiques et migrer les
   consommateurs Account ensemble. Les IDs actuels sont des obligations,
   pas des assertions d'exécution.
2. Produire les enveloppes depuis les runners CI, lier les artefacts à leurs
   producteurs et recalculer les mesures depuis les rapports bruts. Le
   paquet signé contrôle actuellement l'intégrité des fichiers et le run
   référencé, pas leur contenu métier ni leur collecte automatique.
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
