# Rust Workspace Tools

Intégration entre le Cargo Workspace et Nx pour l'exécution unifiée des cibles `check`, `lint`, `test`, `coverage`, `condition` et `mutation`.

- `check-coverage.mjs`: gate de couverture statement/fonction/région (`check-coverage.test.mjs`).
- `check-condition.mjs`: gate de couverture de condition, branche `llvm-cov --branch` (nightly, `check-condition.test.mjs`), réutilise `aggregateCrateCoverage`.
- `check-mutation.mjs`: gate de mutation testing (`check-mutation.test.mjs`).
