# OpenAPI Versionnee

Ce dossier contient les specifications OpenAPI publiques archivees pour les releases stables.

Artefact attendu avant lancement public Drive V1:

```text
docs/api/openapi/drive-public-v1.openapi.json
```

Regles:

- l'artefact est genere depuis le code, pas ecrit a la main;
- l'artefact doit contenir les routes `/v1` publiques;
- la version `info.version` doit correspondre a la release API publique;
- une fois publie, l'artefact est immuable sauf correction documentaire non contractuelle;
- une correction contractuelle publiee exige une entree dans `docs/api/public-api-v1-changelog.md`.

Etat actuel:

- `apps/cloud-service/openapi.json` est l'export technique courant;
- la spec publique versionnee V1 n'est pas encore publiee dans ce dossier.
