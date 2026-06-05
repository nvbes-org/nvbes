# Base Securite et Confidentialite

## Principes

- Hebergement EU-first.
- RGPD by design.
- Minimisation des donnees.
- Securite par defaut.
- Auditabilite claire.
- Transparence sur les sous-traitants.

## Exigences Securite V1

- TLS partout.
- WAF et rate limiting via Cloudflare.
- Chiffrement au repos obligatoire pour PostgreSQL, Object Storage et backups.
- Gestion des cles via KMS ou mecanisme equivalent, avec acces limite aux comptes de service strictement necessaires.
- Rotation documentee des secrets, cles applicatives et tokens d'integration.
- Secrets stockes dans un secret manager ou un coffre dedie, jamais dans le code source ni les images de build.
- Signed URLs courtes pour upload et download.
- Tokens de partage hashes.
- Ne jamais exposer les object keys aux utilisateurs.
- Verifier les permissions workspace cote serveur a chaque action.
- Stocker les fichiers dans l'object storage, pas sur le disque serveur applicatif.
- Separer les environnements et buckets.
- Utiliser des object keys opaques.
- MFA client Owner/Admin recommande; son caractere obligatoire ne fait pas partie du scope V1 tant que la fonctionnalite n'est pas livree et documentee.
- MFA obligatoire des la V1 pour tous les comptes internes et administrateurs d'infrastructure.
- Comptes internes nominatifs, sans compte partage.
- Acces production au moindre privilege.
- Aucun acces direct aux buckets production hors procedure d'incident documentee.
- Logger les actions sensibles.
- Suivre les liens publics actifs.
- Supporter expiration, revocation et journalisation d'acces des liens de partage.
- Imposer une duree maximale aux liens publics en V1.
- Appliquer du rate limiting specifique aux routes publiques de partage.
- Scanner ou mettre en quarantaine les fichiers partages publiquement selon une politique anti-malware documentee.
- Bloquer ou retirer les contenus abusifs selon une procedure d'abus documentee.

## Exigences API Publique V1

- API keys legacy hashees en base.
- API keys affichees une seule fois pour la migration ou les cas historiques.
- Creation de nouvelles API keys desactivee; les nouvelles integrations machine doivent passer par Identity `service accounts`.
- Scopes obligatoires.
- Rate limiting par plan.
- Revocation immediate.
- Rotation supportee.
- Expiration optionnelle selon plan.
- Aucune cle API dans les logs, analytics ou audit metadata non securisee.
- Audit des actions API sensibles.
- Documentation OpenAPI versionnee avant lancement public.

## Exigences Auth et Sessions V1

- Verification email obligatoire avant usage complet du workspace.
- Politique de mot de passe minimale.
- Reset password avec token court, usage unique et expiration.
- Protection brute force sur login, reset password et creation de compte.
- Sessions avec expiration, rotation et invalidation serveur.
- Invalidation des sessions apres changement de mot de passe, changement de role sensible ou suppression de membre.
- Journalisation des login success, login failed, logout et reset password.

## Exigences Privacy V1

- Export des donnees utilisateur.
- Suppression de compte utilisateur.
- Export des donnees workspace.
- Suppression des donnees workspace.
- Regles de retention pour fichiers en corbeille.
- Delai de suppression effective documente pour donnees actives et objets stockes.
- Politique explicite pour les suppressions dans les backups.
- Retention backup documentee avec delai maximal de purge.
- Restauration backup interdite sans rejouer les suppressions deja demandees.
- Documentation des sous-traitants.
- Documentation des categories de donnees.
- Documentation de la retention.
- Procedure de gestion d'incident.

## Exigences Analytics RGPD V1

- Consentement explicite pour analytics marketing non essentiels.
- Product analytics limite aux evenements necessaires a l'amelioration du service.
- Opt-out documente.
- Minimisation stricte des proprietes collectees.
- Pas de noms de fichiers, contenu, emails en clair, tokens ou object keys dans les analytics.
- Retention analytics documentee.
- Attribution marketing sans fingerprinting invasif.
- Sous-traitants analytics documentes.
- Export/suppression des donnees analytics rattachees a un utilisateur si applicable.

## Exigences Audit V1

- Audit logs accessibles uniquement a Owner/Admin.
- Retention audit definie par plan.
- Audit logs append-only au niveau applicatif.
- Journalisation des evenements de securite: login failed, permission denied, share accessed, file downloaded.
- Journalisation des evenements API: api_key.created, api_key.revoked, api.request.denied.
- Export audit disponible pour le owner.
- Toute modification ou suppression d'audit log doit etre interdite hors procedure d'incident.

## Documents Compliance a Maintenir

- Politique de confidentialite.
- Conditions d'utilisation.
- Data processing agreement.
- Liste des sous-traitants.
- Politique de retention.
- Runbook incident.
- Security overview.
- Procedure de demandes RGPD.

## Incident Response Operationnel

- Severites SEV1, SEV2 et SEV3 definies.
- Canal d'alerte interne defini.
- Owner d'incident designe pour chaque incident critique.
- Runbooks pour API down, PostgreSQL down, Object Storage down, bucket public, billing webhook failure et job RGPD failure.
- Criteres de rollback documentes.
- Communication client preparee pour incidents SEV1/SEV2.
- Preservation des logs pendant incident.
- Postmortem obligatoire pour SEV1/SEV2.
- Exercices d'incident planifies avant maturite SOC 2.

## Objectifs de Maturite Plus Tard

- Readiness SOC 2.
- SSO/SAML.
- Audit logs avances.
- Revue des sessions admin.
- Gestion appareils/sessions.
- Controles de chiffrement plus forts.
- Test d'intrusion externe.
