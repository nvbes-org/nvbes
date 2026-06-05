# UI et Design System Guidelines

## Objectif

Définir une UI cohérente, minimale et directement exploitable pour nvbes.

## Règle principale

Chaque élément visuel ou textuel doit justifier sa présence.
Si une phrase, une icône, une bordure ou une section n'apporte pas d'information, de décision ou de sécurité, elle doit être supprimée.

## Principes non négociables

- Utiliser le design system shadcn existant du repo comme base.
- Recomposer avec les composants disponibles avant d'inventer un nouveau pattern.
- Préférer les tokens, variantes et primitives existants.
- Éviter au maximum le CSS custom ad hoc.
- Ne pas créer de système parallèle pour un seul écran.
- Garder une hiérarchie claire et stable.
- Favoriser les formulaires et écrans lisibles en une seule passe.

## Minimalisme

- Zéro texte décoratif.
- Zéro information de remplissage.
- Zéro répétition inutile entre titre, sous-titre et aide contextuelle.
- Une seule idée principale par écran.
- Une action principale maximum par vue.
- Une action secondaire seulement si elle réduit un risque ou évite une perte.
- Tout texte d'aide doit répondre à une question réelle de l'utilisateur.
- Si un texte ne change pas la décision ou l'action, il doit disparaître.

## Composition

- Utiliser `Card` pour les blocs autonomes.
- Utiliser `Separator` seulement si la séparation porte du sens.
- Utiliser `Input`, `Button`, `Progress`, `Skeleton` et les composants shadcn générés localement avant toute alternative.
- Le thème doit venir des fichiers générés par `shadcn` dans chaque app, pas d'un package UI partagé.
- Garder les écrans d'auth et d'onboarding simples, denses et centrés.
- Éviter les colonnes multiples tant que le contenu ne l'exige pas.
- Préférer des espacements réguliers et prévisibles.

## Style visuel

- Base claire, sobre, sans décor inutile.
- Une seule couleur d'accent dominante.
- Bordures fines et surfaces discrètes.
- Radius modéré, pas d'effet gadget.
- Ombres faibles et rares.
- Motion limitée au feedback utile.
- Pas d'illustration décorative par défaut.
- Pas de gradient si le gradient n'apporte pas une fonction.

## Copywriting UI

- Titres courts.
- Verbes d'action clairs.
- Labels explicites.
- Messages d'erreur concrets.
- Messages de succès courts.
- Aide contextuelle uniquement si nécessaire.
- Éviter les phrases marketing dans les vues produit.
- Éviter les abstractions vagues comme "simple", "moderne", "intelligent" si elles n'apportent aucune action.

## États à couvrir

Chaque écran ou composant critique doit gérer:

- chargement;
- vide;
- succès;
- erreur;
- désactivation;
- accès refusé;
- délai ou temporisation.

## Accessibilité

- Toujours associer un label aux champs.
- Ne jamais dépendre uniquement de la couleur pour transmettre un état.
- Garder des contrastes lisibles.
- Support clavier complet.
- Les messages d'état doivent être perceptibles sans lecture de code couleur.

## Do

- Réutiliser les composants du repo.
- Élaguer le texte jusqu'à l'essentiel.
- Faire apparaître l'information utile avant le reste.
- Regrouper les détails secondaires dans un bloc discret.
- Préférer des composants standards et prévisibles.

## Don't

- Ne pas inventer de design system local.
- Ne pas empiler les badges, cartes et séparateurs sans besoin.
- Ne pas sur-documenter l'écran avec du texte de remplissage.
- Ne pas créer de variantes visuelles pour "faire joli" si elles n'aident pas l'utilisateur.
- Ne pas masquer une action essentielle derrière du style.

## Critère de qualité

Un écran est acceptable si:

- on comprend immédiatement ce qu'il faut faire;
- on sait quelle information est critique;
- on ne lit rien d'inutile;
- l'écran reste clair en mobile;
- les composants viennent du système shadcn généré dans l'app.
