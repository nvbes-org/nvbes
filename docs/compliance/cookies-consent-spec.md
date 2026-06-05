# Spec Cookies et Consentement

## Objectif

Definir une implementation compatible CNIL pour les cookies et autres traceurs du site public et, si necessaire, du produit.

## Regles

- Aucun traceur non essentiel ne doit etre depose avant consentement.
- Refuser doit etre aussi simple qu'accepter.
- Le retrait du consentement doit rester accessible a tout moment.
- Les preuves de consentement doivent etre conservables.
- Les finalites doivent etre separees et comprehensibles avant le choix.
- La duree de validite du choix doit etre limitee; cible produit: renouveler la
  demande au plus tard tous les 6 mois ou lors d'un changement de finalite.

## Categories

| Catégorie                      | Consentement requis                | Exemples / Services                                                    |
| ------------------------------ | ---------------------------------- | ---------------------------------------------------------------------- |
| Strictement nécessaires        | Non                                | nvbes Identity (Auth), Stripe (Fraude), Sécurité API                 |
| Mesure d'audience (Exemptée)   | Non (si configuré)                 | PostHog (Anonymisé, proxyfié conforme CNIL)                            |
| Product Analytics              | Oui                                | PostHog (Full sessions, replay, heatmaps)                              |
| Performance / Erreurs          | Oui sauf qualification stricte     | Sentry browser/SW si non strictement technique; diagnostics techniques exemptes seulement si minimises |

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
- identifiant pseudonyme ou compte authentifie lorsque disponible;
- source du choix (`identity-web`, `drive-web`, service worker sync).

## Regles Sentry

- Sentry browser et service worker restent bloques par defaut tant que le vendor
  `sentry` n'est pas accepte.
- Une exemption "strictement technique" doit etre documentee avant activation
  sans consentement: finalite securite/diagnostic, minimisation, pas de replay,
  pas d'identifiant direct, `sendDefaultPii=false`, scrubber actif, duree courte.
- A defaut de cette documentation, Sentry est traite comme performance soumis au
  consentement prealable.

## Gouvernance

- Toute nouvelle librairie frontend doit etre qualifiee avant integration.
- Toute fonctionnalite de session replay ou equivalent doit faire l'objet d'une revue juridique et privacy.
