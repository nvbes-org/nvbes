# Account Quality Tools

> **Statut : harness historique / diagnostique.** Ne prononce plus GO pour le
> socle V1. La preuve canonique est
> [`docs/testing/v1/account.json`](../../docs/testing/v1/account.json).

Outils hérités de conformité release, DAST et load pour l'ancien portefeuille
Account (`account-web` / `account-worker` archivés). Les scripts encore
référencés par des workflows actifs (ex. DAST) restent exécutables, mais le
runtime lean se valide via `account-service` + `account-sdk-core`.
