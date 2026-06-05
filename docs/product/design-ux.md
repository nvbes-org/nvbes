# Design, UX et Design System

Le contrat d'execution UI detaille et minimaliste est defini dans [UI et Design System Guidelines](ui-guidelines.md).

## Decision

nvbes Drive V1 adopte une direction `B2B sobre et dense`.

L'interface doit se comporter comme un outil de travail quotidien: rapide a scanner, fiable, calme, precise et peu decorative.

## Fondation Design System

nvbes Design System V1 est construit sur:

- `shadcn/ui` officiel pour les composants de base copies dans le repo.
- `Radix UI` pour les primitives accessibles.
- `Tailwind CSS` et CSS variables pour les tokens.
- `lucide-react` pour les icones.

Regle:

- `shadcn/ui` est la base technique, pas l'identite visuelle.
- nvbes possede et customise le code des composants.
- Les composants metier nvbes sont construits au-dessus de `shadcn/ui`.
- Les plugins/templates tiers ne doivent pas devenir une dependance critique du design system.

## Direction Artistique

Personnalite:

- Sobre.
- Dense.
- Professionnelle.
- Precise.
- Fiable.
- Calme.
- Europeenne sans imagerie decorative.

Anti-patterns:

- Look startup generique.
- Degrades violets/bleus dominants.
- Illustrations SaaS abstraites.
- Cartes flottantes partout.
- Dark mode par defaut.
- Interface marketing apres login.
- UI trop consumer/lifestyle.

Style cible:

- Fond blanc froid ou gris tres clair.
- Texte graphite ou noir doux.
- Bordures fines.
- Radius faible: 6 a 8px.
- Densite moyenne-haute.
- Iconographie sobre.
- Motion minimale, uniquement pour feedback utile.
- Securite visible mais discrete.

Palette de depart:

- Base: blanc froid, gris brume, graphite.
- Accent principal: bleu petrole ou vert profond.
- Success: vert discret.
- Warning: ambre.
- Danger: rouge sobre.

Typographie recommandee:

- Interface: `IBM Plex Sans`, `Source Sans 3`, `Geist` ou equivalent lisible.
- Mono: `IBM Plex Mono` ou `JetBrains Mono` pour IDs, audit et donnees techniques.

## Navigation V1

Libelles visibles:

- Fichiers.
- Liens partages.
- Corbeille.
- Membres.
- Facturation.
- Securite.
- API.
- Compte.

Taxonomie:

| Terme interne | Terme UI           |
| ------------- | ------------------ |
| Workspace     | Espace             |
| Owner         | Proprietaire       |
| Admin         | Admin              |
| Member        | Membre             |
| Viewer        | Lecteur            |
| Billing       | Facturation        |
| Audit         | Journal d'activite |
| Share link    | Lien partage       |
| Trash         | Corbeille          |

## Structure UI V1

### Layout App

- Sidebar stable.
- Header compact.
- Breadcrumb dans les vues fichier.
- Toolbar contextuelle.
- Zone de contenu principale dense.
- Panneau details optionnel plus tard.
- Quota toujours accessible.

### Vue Fichiers

Vue par defaut: liste/table dense.

Colonnes:

- Nom.
- Statut.
- Proprietaire.
- Taille.
- Modifie le.
- Partage.

Actions principales:

- Importer.
- Nouveau dossier.
- Rechercher.
- Ouvrir.
- Telecharger.
- Partager.
- Deplacer.
- Renommer.
- Mettre en corbeille.

### Vue Liens Partages

Colonnes:

- Fichier ou dossier.
- Cree par.
- Permission.
- Expiration.
- Acces/telechargements.
- Statut.

Actions:

- Copier le lien.
- Modifier l'expiration.
- Revoquer.

### Vue Facturation

Hierarchie:

1. Plan actuel.
2. Usage stockage.
3. Utilisateurs inclus/utilises.
4. Prochaine facture estimee.
5. Usage additionnel.
6. Alertes quota.
7. Moyen de paiement.
8. Factures.
9. Actions upgrade/downgrade.

### Vue Securite

Sections:

- Liens publics actifs.
- Dernieres actions sensibles.
- Sessions actives.
- Policies de partage.
- Export du journal d'activite.

### Vue API

Sections:

- Liste des cles API.
- Nom de la cle.
- Prefixe visible.
- Scopes.
- Statut.
- Expiration.
- Derniere utilisation.
- Creee par.
- Actions: creer, revoquer, regenerer.
- Lien vers documentation API.

Copy de securite:

```text
Les cles API donnent acces aux fichiers de cet espace. Conservez-les comme des mots de passe.
```

## Etats Critiques V1

Chaque etat critique doit avoir:

- Titre clair.
- Explication courte.
- Action principale.
- Action secondaire si utile.
- Niveau visuel: info, success, warning, danger.
- Message accessible sans dependance exclusive a la couleur.

Etats a couvrir:

- Drive vide.
- Recherche sans resultat.
- Upload en cours.
- Upload echoue.
- Upload interrompu.
- Quota presque atteint.
- Quota depasse.
- Permission refusee.
- Lien expire.
- Lien revoque.
- Fichier en quarantaine.
- Billing echoue.
- Trial expire.
- Workspace suspendu.
- Corbeille vide.
- Object Storage indisponible ou mode degrade.
- Cle API creee.
- Cle API revoquee.
- Scope API insuffisant.
- Rate limit API atteint.

Ces etats doivent etre couverts par la strategie UI definie dans [Strategie UI testing](../testing/ui-testing.md).

## Onboarding et Activation

Le funnel complet, les events et les KPIs sont definis dans [Marketing, Conversion et Product Analytics](marketing-growth.md).

Objectif UX:

- Premier fichier importe en moins de 2 minutes.
- Activation cible: 60% des nouveaux workspaces uploadent un fichier en moins de 10 minutes.

Ordre recommande:

1. Creer compte.
2. Verifier email si necessaire.
3. Nommer l'espace.
4. Arriver directement dans Fichiers.
5. Importer un premier fichier.
6. Proposer de creer un lien securise.
7. Proposer d'inviter un membre.
8. Afficher le trial et quota de facon claire mais non bloquante.

Regle:

- Ne pas forcer le choix du plan avant le premier upload si l'essai est sans carte.

## Design System Components

Composants de base issus de `shadcn/ui`:

- Button.
- IconButton.
- Input.
- SearchInput.
- Select.
- Checkbox.
- DropdownMenu.
- Dialog.
- Sheet.
- Table.
- Sidebar.
- Breadcrumb.
- Badge.
- Progress.
- Alert.
- Toast/Sonner.
- Tooltip.
- Tabs.
- Separator.
- Skeleton.
- Command.

Composants nvbes:

- FileRow.
- FileTable.
- FileIcon.
- UploadDropzone.
- UploadProgress.
- QuotaMeter.
- ShareLinkModal.
- InviteMemberModal.
- PermissionBadge.
- RoleBadge.
- PlanBadge.
- BillingUsagePanel.
- AuditEventRow.
- ApiKeyTable.
- ApiScopeSelector.
- ApiKeyCreatedDialog.
- EmptyState.
- CriticalStateBanner.
- ConfirmDialog.

## Copywriting

Voix produit:

- Calme.
- Directe.
- Precise.
- Non anxiogene.
- Sans jargon inutile.
- Sans promesse de securite excessive.

Copy marketing:

- Hero, pricing, FAQ et preuves de confiance sont documentes dans [Marketing, Conversion et Product Analytics](marketing-growth.md).
- Le copy marketing doit convertir sans surpromettre: hebergement europeen, controle des liens et simplicite d'equipe sont les preuves principales.

Principes:

- Dire ce qui se passe.
- Dire ce que l'utilisateur peut faire.
- Rendre les limites visibles.
- Ne pas dramatiser.
- Ne pas cacher les couts ou quotas.

CTA standards:

- Importer des fichiers.
- Creer un dossier.
- Creer un lien securise.
- Inviter un membre.
- Voir l'usage.
- Changer de plan.
- Revoquer le lien.
- Restaurer.
- Supprimer definitivement.

Exemples de messages:

```text
Lien expire
Ce lien n'est plus accessible. Demandez un nouveau lien a la personne qui l'a partage.
```

```text
Permission refusee
Vous n'avez pas les droits necessaires pour effectuer cette action.
```

```text
Upload interrompu
L'import a ete interrompu. Vous pouvez reessayer sans perdre les fichiers deja envoyes.
```

```text
Fichier en quarantaine
Ce fichier est en cours de verification avant partage public.
```

```text
Quota presque atteint
Vous utilisez 90% du stockage inclus dans votre plan.
```

## Accessibilite

Contraintes V1:

- Viser WCAG AA.
- Navigation clavier complete.
- Focus visible sur tous les composants interactifs.
- Contrastes suffisants.
- Labels explicites.
- Dropzone utilisable sans drag-and-drop.
- Boutons icon-only avec `aria-label`.
- Progress upload annonce aux technologies d'assistance.
- Reduction motion respectee.
- Taille tactile minimale correcte.
- Aucun message uniquement par couleur.

Verification attendue:

- checks automatises sur les vues critiques;
- revue manuelle clavier;
- revue manuelle lecteur d'ecran sur les flows critiques;
- verification responsive minimale sur desktop et mobile.

## Livrables Design Avant Implementation UI

- Direction artistique validee.
- Taxonomie UI validee.
- UX map V1.
- Wireframes V1.
- Fondations du design system.
- Inventaire composants.
- Copy deck V1.
- Checklist accessibilite.
- Prototype clickable.
- Test utilisabilite avec 3 a 5 utilisateurs cibles.

Le protocole de recherche et les taches minimum sont definis dans [Protocole UX testing](../testing/ux-testing.md).
