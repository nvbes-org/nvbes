# Spec Cookies et Consentement

## Objectif

Definir une implementation compatible CNIL pour les cookies et autres traceurs du site public et, si necessaire, du produit.

## Regles

- Aucun traceur non essentiel ne doit etre depose avant consentement.
- Refuser doit etre aussi simple qu'accepter.
- Le retrait du consentement doit rester accessible a tout moment.
- Les preuves de consentement doivent etre conservables.

## Categories

| Catégorie                      | Consentement requis                | Exemples / Services                                                    |
| ------------------------------ | ---------------------------------- | ---------------------------------------------------------------------- |
| Strictement nécessaires        | Non                                | nvbes Identity (Auth), Stripe (Fraude), Sécurité API                 |
| Mesure d'audience (Exemptée)   | Non (si configuré)                 | PostHog (Anonymisé, proxyfié conforme CNIL)                            |
| Product Analytics              | Oui                                | PostHog (Full sessions, replay, heatmaps)                              |
| Performance / Erreurs          | Non (si strictement technique)     | Sentry (Diagnostic technique uniquement)                              |

## Exigences UI

- Bouton `Accepter` et bouton `Refuser` au premier niveau.
- Lien ou bouton `Personnaliser`.
- Texte clair sur les finalites.
- Pas de dark patterns.

## Preuves minimales

- version de la banniere;
- horodatage;
- choix utilisateur;
- perimetre de finalites;
- preuve technique de configuration.

## Gouvernance

- Toute nouvelle librairie frontend doit etre qualifiee avant integration.
- Toute fonctionnalite de session replay ou equivalent doit faire l'objet d'une revue juridique et privacy.
