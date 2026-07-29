# Exercices de compromission sécurité

Chaque scénario est joué au moins une fois par an et après une modification
majeure de sa frontière de confiance. Une simulation sur table est insuffisante :
au moins une étape de révocation, restauration ou bascule doit être exécutée
dans un environnement isolé. Les preuves ne contiennent ni secrets ni données
personnelles.

## Identity Provider compromise

Déclencheurs : signature JWT suspecte, refresh token réutilisé, session volée,
création de passkey inconnue ou élévation non autorisée.

1. Déclarer SEV1 et geler les changements d’identité.
2. Désactiver la clé de signature suspecte, publier un JWKS sans cette version et
   invalider sessions, refresh families, device grants et codes actifs.
3. Exiger un nouveau step-up résistant au phishing pour les comptes privilégiés.
4. Comparer PostgreSQL, Redis, audit signé externe et journaux Cloudflare.
5. Reconstituer la fenêtre d’impact par tenant sans exposer les identités dans le
   canal incident.
6. Restaurer le service avec une nouvelle version KMS et vérifier les clients
   OIDC critiques.

Succès : ancien token refusé, nouvelle clé vérifiable, sessions révoquées, chaîne
d’audit vérifiée et notification juridique évaluée.

## KMS compromise

1. Désactiver la version de clé ; ne pas supprimer le matériel.
2. Révoquer l’identité IAM ayant appelé KMS et exporter l’Audit Trail.
3. Créer une nouvelle version protégée avec double contrôle.
4. Re-signer uniquement les nouveaux artefacts. Les anciens anchors restent
   conservés avec leur key ID et sont qualifiés dans le registre d’incident.
5. Vérifier qu’aucune clé privée n’a été exportée dans les workloads.

Succès : appels avec l’ancien principal refusés, nouvelle signature vérifiée par
la clé publique publiée et historique WORM intact.

## CI compromise

1. Suspendre les déploiements et révoquer OIDC, deploy keys et tokens runners.
2. Identifier le dernier commit et artefact de confiance.
3. Comparer provenance, digest, SBOM, signature et journaux d’environnement.
4. Recréer runners et credentials depuis une racine saine.
5. Rebuilder sans cache, signer et redéployer l’artefact connu.

Succès : aucun ancien credential accepté, artefact reproductible et preuve de
provenance attachée au postmortem.

## Database or tenant isolation compromise

1. Passer le trafic en lecture seule ou isoler le service concerné.
2. Révoquer le rôle applicatif et capturer les connexions actives.
3. Vérifier RLS, rôle non-owner, contexte tenant et requêtes cross-tenant.
4. Comparer WAL/PITR, audit local et anchors externes.
5. Restaurer dans un réseau isolé, mesurer l’exposition et tester les limites de
   tenant avant réouverture.

Succès : tests cross-tenant négatifs, rôle minimal rétabli, intégrité des données
confirmée et RPO/RTO mesurés.

## Preuves d’exercice

Chaque exercice produit : participants et rôles, chronologie, décisions,
commandes expurgées, mesures de détection/containment/recovery, écarts, owner et
date limite. Un exercice échoué bloque la release qui dépend du contrôle.
