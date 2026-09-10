# Identity Web : restauration du design archivé

La référence visuelle demandée est `archive/apps/identity-web`.
Le runtime actif reste `apps/identity-web`.

## Éléments repris

- Feuille de styles archivée : couleurs claires et sombres, Geist, rayons,
  animations du logo et transitions.
- `AuthPageShell`, `AuthBrandWordmark`, `LoginProgress` et le composant Card
  proviennent du registry archivé du projet.
- Le panneau de marque reprend les dimensions, marges, titres et descriptions
  archivés. Connexion, récupération, déconnexion et page d'entrée partagent
  désormais cette composition.
- Le formulaire retrouve les étapes email puis mot de passe, les champs
  arrondis de 44 px et les boutons arrondis alignés à droite.

## Adaptation au runtime actif

L'identification est une étape locale : aucune recherche de compte ni requête
réseau avant la soumission du mot de passe. Retour conserve l'email et détruit
le champ secret. La soumission efface le secret avant l'appel au contrôleur.

Les contrôleurs OAuth, passkeys, MFA, récupération et logout restent actifs.
L'identifiant du client reste visible avant le consentement. La connexion
directe par passkey et l'annulation restent proposées.

Ce port restaure l'habillage archivé ; il ne constitue pas une preuve de
parité pixel par pixel pour tous les écrans. Les contenus et contrôles des
capacités actives diffèrent de l'ancien runtime. Les liens d'inscription,
de mot de passe oublié et légaux de l'archive ne sont pas repris sans
destination active configurée. Aucun parcours public supplémentaire n'est ouvert.

## Validation

- Nx : tests, typecheck, lint et build Identity Web.
- Test du passage email → mot de passe → retour, sans appel réseau à
  l'identification, puis soumission avec effacement du secret.
- Fixture HTTPS avec services réels : mot de passe, TOTP, passkey et consentement.
- Fixture Account : callback, profil, déconnexion RP, retour validé,
  refus du retour falsifié et révocation des accès Billing.
- Captures desktop 1280 × 800 et mobile 390 × 844 ; absence de débordement.

Les authentificateurs de la fixture sont synthétiques. Aucune validation
matérielle ni mise en production n'est déduite de ces tests.
