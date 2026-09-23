# Charters d'acceptation Account (V1)

> **Statut : socle V1 B2C.** Les paquets Alpha/Beta/Cloud/Enterprise historiques
> sont hors périmètre. L'acceptation V1 est une session opérateur solo contre
> le runtime `account-service`, documentée dans le
> [runbook Platform Operations](../operations/platform-operations-manual-runbook.md).

## Règles communes

Chaque session produit :

- environnement (isolated ou staging) et SHA candidat ;
- commandes exécutées et leur sortie non secrète ;
- décision, owner et date ;
- aucune donnée personnelle réelle.

Une anomalie critique (auth, perte de données, outbox non publiable, fermeture
irréversible incorrecte) arrête la promotion.

## Scénarios V1 obligatoires

1. **Profil / préférences** — `GET/PUT` profil et préférences avec token
   audience `nvbes-account-service` et scopes `account:read|write`.
2. **Consents** — grant puis revoke d'une acceptation légale ; présente dans
   l'export RGPD.
3. **Équipes** — create, join via code, list members, leave membre, retrait
   membre par owner ; owner avec membres ne peut pas leave (`409`).
4. **Export** — demande avec step-up, job `process-privacy-jobs`, téléchargement
   document, expiration 24 h.
5. **Fermeture** — demande avec step-up, fenêtre 7 jours, annulation, puis
   redaction Account-local.
6. **Outbox** — événements enqueued puis `publish-outbox` (ou boucle runtime)
   pose `published_at`.

## Preuve

- Automatisé : suites de [`docs/testing/v1/account.json`](v1/account.json).
- Manuel : checklist ci-dessus cochée dans le runbook, liée au SHA candidat.
- Le manifeste historique et `tools/account-quality` ne suffisent pas pour GO.
