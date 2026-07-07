# Architecture Technique

## Stack

- Monorepo.
- Backend Rust.
- Frontend TypeScript.
- Vite.
- React.
- TanStack Router pour les routes frontend.
- TanStack Query pour l'etat serveur frontend.
- Effect pour les workflows frontend multi-etapes.
- Cloudflare pour l'edge et la protection.
- Scaleway pour compute, PostgreSQL, Redis, object storage, backups et hebergement des donnees en Europe.

## Modele de Deploiement Initial

La V1 doit demarrer avec un backend modulaire unique, pas un systeme microservices distribue.

Le backend doit etre structure par domaines pour permettre une extraction future quand un module le justifie.

## Infrastructure

Les exigences detaillees de reseau, CI/CD, IaC, backups, observabilite, migrations et runbooks sont definies dans [Infrastructure, Network et DevOps](infrastructure-devops.md).

### Cloudflare

- DNS.
- TLS.
- WAF.
- Rate limiting.
- Bot protection.
- CDN pour les assets frontend statiques.
- Zero Trust pour les acces internes/admin des la V1.

### Scaleway

- Compute pour l'API Rust et les workers.
- PostgreSQL manage pour les metadonnees.
- Object Storage compatible S3 pour les fichiers utilisateurs.
- Backups.
- Logs et metriques.

## Exigences Operationnelles V1

- Topologie reseau documentee avant production.
- Environnements `development`, `staging` et `production` separes.
- Infrastructure critique geree en IaC.
- CI/CD avec tests, scans, artefacts immutables, staging et approval production.
- Migrations PostgreSQL versionnees et compatibles avec rollback.
- Backups chiffres avec RPO/RTO et test de restauration.
- Observabilite avec logs structures, metriques, traces ou correlation IDs, dashboards et alertes.
- Workers monitorables, idempotents, avec retries, backoff et dead-letter.
- Object Storage prive avec CORS limite, lifecycle rules et nettoyage des uploads incomplets.
- Runbooks incident pour API, PostgreSQL, Object Storage, billing, jobs RGPD et bucket public.

La couverture attendue par type de test est definie dans [Strategie de test](../testing/test-strategy.md).

## Domaines Backend

Account expose les domaines compte sous `/api/v1` et OAuth sous `/oauth`.
Cloud expose les routes produit Cloud a la racine du service Cloud, plus l'API
publique versionnee sous `/v1`.

- Auth.
- Workspaces.
- Membres.
- Fichiers et dossiers.
- Sessions d'upload.
- URLs de download.
- Liens de partage.
- Quotas.
- Billing.
- Audit.
- Privacy.

## Modele de Stockage

- Les fichiers sont stockes dans Scaleway Object Storage.
- PostgreSQL stocke uniquement les metadonnees.
- Les noms de fichiers controles par l'utilisateur ne doivent jamais definir les chemins object storage.
- Les object keys doivent etre opaques et generees par le backend.
- Les buckets sont separes par environnement: development, staging, production.

Exemple d'object key:

```text
workspaces/{workspace_id}/objects/{storage_object_id}/{version_id}
```

## Flow Upload

1. Le frontend demande une session d'upload a l'API.
2. L'API verifie l'auth, les permissions, les limites du plan et le quota.
3. L'API cree un storage object en etat pending.
4. L'API retourne une signed upload URL courte duree.
5. Le frontend upload directement vers l'object storage.
6. Le frontend confirme la fin avec taille et checksum.
7. L'API verifie l'objet directement cote Object Storage via metadata ou requete HEAD.
8. L'API marque l'objet comme active.
9. L'API met a jour le quota et cree les audit events.

Contraintes:

- La session d'upload expire rapidement.
- Une session d'upload est usage unique.
- La taille attendue est enregistree avant generation de la signed URL.
- Le checksum attendu est verifie quand le client le fournit.
- L'objet final doit correspondre a la taille attendue, au workspace et a la session active.
- Les uploads incomplets ou expires sont purges par worker.
- Les quotas sont reserves ou revalides avant activation pour eviter les contournements par concurrence.
- Les fichiers partages publiquement passent par la politique anti-malware avant exposition.

Scenarios a couvrir au minimum:

- activation d'objet apres upload valide;
- refus d'activation si taille ou checksum invalide;
- purge des uploads expires;
- comportement concurrent sur revalidation quota.

## Flow Download

1. Le frontend demande une URL de download a l'API.
2. L'API verifie l'auth et les permissions.
3. L'API retourne une signed download URL courte duree.
4. Le frontend telecharge depuis l'object storage.
5. L'API enregistre les evenements d'audit et d'usage.

## Flow Partage Public

1. Le visiteur ouvre `/public/shares/{token}`.
2. L'API hash le token et resout le lien de partage.
3. L'API verifie expiration, revocation et limites de download.
4. L'API retourne les metadonnees publiques.
5. Le visiteur demande une URL de download.
6. L'API retourne une signed download URL courte duree.

Contraintes:

- Aucun lien public sans expiration en V1.
- Une duree maximale de lien est definie par plan.
- Les routes publiques de partage ont un rate limiting dedie.
- Chaque acces a un lien public cree un audit event ou un usage event.
- Les liens publics peuvent etre revoques immediatement.
- La protection par mot de passe ou code d'acces est hors scope V1.

Scenarios a couvrir au minimum:

- acces lien valide;
- acces lien expire;
- acces lien revoque;
- limitation download si activee;
- absence d'exposition de donnees sensibles dans la route publique.

## Workers

Les workers gerent:

- Purge de corbeille.
- Nettoyage des uploads expires.
- Recalcul des quotas.
- Webhooks billing.
- Snapshots d'usage.
- Exports RGPD.
- Suppression de compte ou workspace.
- Nettoyage des liens expires.
- Emails transactionnels.

La V1 utilise une queue dediee pour les jobs; PostgreSQL ne doit pas servir de file de polling.

Garanties requises:

- Jobs idempotents.
- Retries limites avec backoff.
- Timeout par type de job.
- Locking pour eviter les executions concurrentes non voulues.
- Statuts `pending`, `running`, `succeeded`, `failed`, `dead-letter`.
- Monitoring de la queue, des echecs et des jobs bloques.
- Alertes sur jobs critiques: billing, RGPD, quotas et purge.

Scenarios a couvrir au minimum:

- retries et backoff;
- idempotence d'un job relance;
- non double execution concurrente;
- reprise apres crash;
- dead-letter sur echec definitif.

### Validation RGPD sur staging

Avant de considerer les procedures export/suppression RGPD comme valides, elles doivent etre verifiees sur `staging` avec le chemin complet API -> worker -> email.

Checklist minimale:

1. Creer ou reutiliser un compte de test avec une boite de reception accessible.
1. Appeler `POST /api/v1/auth/me/export`.
1. Verifier qu'une privacy request est creee et qu'un job worker `privacy.account_export` est enfile.
1. Verifier qu'un job email `email.send` est enfile avec `business_type = data_export`.
1. Verifier que le worker consomme le job et le marque `succeeded`.
1. Verifier la reception de l'email d'export et son destinataire.
1. Rejouer l'operation si necessaire pour confirmer l'idempotence et l'absence de double envoi.
1. Effectuer le step-up recent requis, puis appeler `POST /api/v1/auth/me/delete`.
1. Verifier qu'une privacy request est creee et qu'un job worker `privacy.account_delete` est enfile.
1. Verifier que le worker supprime les memberships et marque le user `deleted`.
1. Verifier que les sessions sont revoquees et qu'aucune erreur worker/email n'apparait dans les logs.
1. Tester les garde-fous: rate limit export, refus sans step-up recent, refus si le compte possede encore des workspaces, refus workspace sous legal hold.

Cette validation doit etre documentee dans le runbook d'exploitation si un comportement change dans la chaine RGPD.
