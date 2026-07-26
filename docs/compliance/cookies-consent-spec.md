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

| Catégorie                    | Consentement requis             | Exemples / Services                                                                                    |
| ---------------------------- | ------------------------------- | ------------------------------------------------------------------------------------------------------ |
| Strictement nécessaires      | Non                             | nvbes Identity (Auth), Stripe (Fraude), Sécurité API                                                   |
| Mesure d'audience (Exemptée) | Non (si configuré et documenté) | Mesure strictement anonymisée, proxyfiée, sans cross-site ni replay                                    |
| Product Analytics            | Oui                             | PostHog `posthog_product_analytics`                                                                    |
| Heatmaps / Autocapture       | Oui, finalité séparée           | PostHog `posthog_autocapture_heatmaps`, routes sensibles bloquées                                      |
| Session Replay               | Oui, finalité séparée           | PostHog `posthog_session_replay`, masquage texte/input fort                                            |
| Surveys / Feedback           | Oui, finalité séparée           | PostHog `posthog_surveys_feedback`, post-auth hors pages sensibles                                     |
| Feature Flags / Experiments  | Oui, finalité séparée           | PostHog `posthog_feature_flags`, flags non critiques uniquement                                        |
| Error Tracking PostHog       | Oui, finalité séparée           | PostHog `posthog_error_tracking`, scrubber actif                                                       |
| Performance / Erreurs        | Oui sauf qualification stricte  | Sentry browser/SW si non strictement technique; diagnostics techniques exemptes seulement si minimises |

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
- source du choix (`account-web`, `cloud-web`, service worker sync).

## Regles Sentry

- Sentry browser et service worker restent bloques par defaut tant que le vendor
  `sentry` n'est pas accepte.
- Une exemption "strictement technique" doit etre documentee avant activation
  sans consentement: finalite securite/diagnostic, minimisation, pas de replay,
  pas d'identifiant direct, `sendDefaultPii=false`, scrubber actif, duree courte.
- A defaut de cette documentation, Sentry est traite comme performance soumis au
  consentement prealable.

## Regles PostHog

- Stockage consentement courant: `nvbes.tracking-consent.v4`.
- Version de notice backend courante:
  `cookie-notice-2026-07-20`.
- Les finalites PostHog sont separees:
  `posthog_product_analytics`, `posthog_autocapture_heatmaps`,
  `posthog_session_replay`, `posthog_surveys_feedback`,
  `posthog_error_tracking`, `posthog_feature_flags`.
- Une finalite non utilisee par le produit reste desactivee et n'est pas
  proposee dans la banniere. Son activation exige une nouvelle version de
  notice et un nouveau choix.
- Les choix v1, v2 et v3 ne sont jamais étendus à la notice v4. Une nouvelle
  décision est demandée avant toute activation optionnelle.
- Le retrait a chaud doit stopper capture, replay, surveys, polling flags et
  purger cookies/storage PostHog.
- Interdit dans PostHog: email, nom, nom de fichier, object key, token, signed
  URL, payload utilisateur, contenu de fichier, UUID brut.
- Les distinct IDs et group IDs doivent etre pseudonymises par HMAC avec un
  salt analytics.
- Les feature flags PostHog ne doivent jamais piloter auth, securite, billing
  enforcement ou autorisation d'acces.

## Gouvernance

- Toute nouvelle librairie frontend doit etre qualifiee avant integration.
- Toute fonctionnalite de session replay ou equivalent doit faire l'objet d'une revue juridique et privacy.
