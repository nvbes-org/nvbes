# Strategie UI Testing

## Objectif

Definir comment verrouiller les composants, etats critiques et comportements visuels essentiels.

## Niveaux

### Tests de composants

Couvrir au minimum:

- rendu des composants metier critiques;
- variantes de props;
- etats loading, error, empty, success;
- callbacks et interactions principales;
- accessibilite basique des composants interactifs.

Composants prioritaires V1:

- UploadDropzone;
- UploadProgress;
- FileTable;
- ShareLinkModal;
- InviteMemberModal;
- QuotaMeter;
- BillingUsagePanel;
- ApiKeyTable;
- CriticalStateBanner;
- ConfirmDialog.

### Tests de pages/vues

Vues prioritaires:

- Fichiers;
- Liens partages;
- Facturation;
- Securite;
- API;
- Compte.

### Verifications visuelles

Objectif:

- Detecter les regressions de layout, densite, hiérarchie, etats critiques et responsive.

Priorites:

- desktop principal;
- largeur laptop moyenne;
- mobile pour flows publics et onboarding minimum.

## Accessibilite automatisee minimale

- check axe ou equivalent sur pages critiques;
- navigation clavier sur modales et dialogues;
- presence de labels et noms accessibles;
- verification absence de message uniquement couleur.

## Declencheurs

- modification composant design system;
- modification composant metier critique;
- ajout d'etat critique;
- changement copy ayant impact structurel;
- changement responsive.
