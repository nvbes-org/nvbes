# Permissions

> **Statut : modèle produit historique à réévaluer.** Les primitives Identity,
> Account et équipes du socle V1 doivent rester réutilisables et sans gouvernance
> Enterprise. Les permissions Cloud/Drive ci-dessous sont futures. Voir la
> [direction produit](nvbes-product-strategy.md).

## Principe

Les permissions produit V1 sont principalement au niveau du workspace.

Elles s'executent toutefois dans un cadre identity plus large:

- `user` global;
- `tenant` obligatoire;
- `organization` optionnelle;
- `workspace` comme contexte produit;
- policies de securite heritees `system -> tenant -> organization -> workspace`.

Il n'y a pas de permissions par dossier ou par fichier en V1.

## Roles

- Owner.
- Admin.
- Member.
- Viewer.

Libelles UI:

- Owner: Proprietaire.
- Admin: Admin.
- Member: Membre.
- Viewer: Lecteur.

## Owner

Peut:

- Gerer les parametres du workspace.
- Gerer la facturation.
- Inviter et retirer des membres.
- Changer les roles.
- Voir les audit events.
- Creer des dossiers.
- Uploader des fichiers.
- Renommer, deplacer, mettre en corbeille, restaurer et supprimer des fichiers.
- Supprimer definitivement des fichiers.
- Creer et revoquer des liens de partage.
- Exporter ou supprimer les donnees du workspace.

## Admin

Peut:

- Inviter et retirer des members et viewers.
- Voir les audit events.
- Creer des dossiers.
- Uploader des fichiers.
- Renommer, deplacer, mettre en corbeille, restaurer et supprimer des fichiers.
- Creer et revoquer des liens de partage.

Ne peut pas:

- Gerer la facturation.
- Retirer le owner.
- Supprimer le workspace.

## Member

Peut:

- Creer des dossiers.
- Uploader des fichiers.
- Renommer ses propres fichiers.
- Deplacer ses propres fichiers.
- Mettre ses propres fichiers en corbeille.
- Creer des liens de partage uniquement pour ses propres fichiers si la policy workspace l'autorise.
- Telecharger des fichiers.

Ne peut pas:

- Gerer les membres.
- Voir la facturation.
- Voir l'audit complet.
- Supprimer definitivement des fichiers.
- Partager des fichiers crees par d'autres membres sans validation Owner/Admin.

## Viewer

Peut:

- Voir les fichiers.
- Telecharger les fichiers.

Ne peut pas:

- Uploader des fichiers.
- Modifier des fichiers.
- Partager des fichiers.
- Supprimer des fichiers.

## Actions Toujours Auditees

- `auth.login_failed`
- `permission.denied`
- `member.invited`
- `member.removed`
- `member.role_changed`
- `file.downloaded`
- `file.deleted_permanently`
- `share_link.created`
- `share_link.accessed`
- `share_link.revoked`
- `billing.updated`
- `workspace.export_requested`
- `workspace.delete_requested`

## Policies Workspace V1

- `member_can_create_share_links`: desactivee par defaut sur Team et Workspace.
- `default_share_link_ttl_days`: 7 jours.
- `max_share_link_ttl_days`: defini par plan.
- `require_admin_approval_for_member_share`: activee par defaut si le partage membre est autorise.

Ces policies sont des parametres de workspace administres par Owner, et supportes par le modele de donnees V1.
