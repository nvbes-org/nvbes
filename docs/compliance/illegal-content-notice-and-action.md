# Procedure de Signalement de Contenu Illicite

## Objectif

Définir le mécanisme futur de signalement et de traitement des contenus
potentiellement illicites si un produit nvbes héberge ou partage du contenu.

La V1 du socle n'héberge aucun produit de contenu et n'automatise donc pas sa
modération. Toute demande reçue est traitée manuellement par `platform_owner`,
avec motif, preuve, mesure réversible et audit. Cette procédure devient active
pour un produit seulement après revue de son applicabilité légale et de son
budget.

## Canal de signalement

Le service doit proposer un canal electronique accessible permettant de signaler un contenu ou un lien.

Le formulaire ou canal doit permettre de fournir au minimum:

- l'URL ou l'identifiant du lien / contenu;
- le motif du signalement;
- les dispositions ou faits invoques si disponibles;
- les coordonnees du declarant;
- une declaration de bonne foi.

## Workflow

1. Enregistrer manuellement le signalement dans un dossier Platform Operations.
2. Qualifier s'il concerne un contenu illicite, un abus, ou un incident de securite.
3. Preserver les preuves.
4. Evaluer rapidement le caractere manifestement illicite ou le besoin d'escalade.
5. Prendre manuellement la mesure appropriee: maintien, limitation, retrait,
   suspension du lien ou gel temporaire. Une automatisation ne peut pas décider
   seule d'une mesure irréversible.
6. Informer les parties pertinentes lorsque cela est approprie.
7. Journaliser la decision et sa base.

## Journal minimal

- identifiant du signalement;
- date/heure;
- contenu ou lien vise;
- type de risque;
- opérateur;
- decision;
- base legale ou contractuelle;
- date de cloture.

## Coordination

- Les contenus lies a un risque penal ou de securite majeur doivent etre escalades immediatement.
- Si l'incident implique aussi des donnees personnelles, ouvrir en parallele le workflow de violation.
