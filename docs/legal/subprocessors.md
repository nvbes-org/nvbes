# Liste des Sous-traitants

## Statut

Liste des prestataires datée du 2026-08-12 ; identité de l’exploitant mise à jour
le 2026-09-07, sans nouvelle vérification des prestataires à cette date.

Les services Nvbes sont exploités par Rayane Guemmoud EI (nom commercial
OTAKIMI), identifié dans les [mentions légales](./site-legal-notice.md).

## Regles de publication

Pour chaque sous-traitant, documenter:

- le service fourni;
- les categories de donnees traitees;
- la localisation principale;
- un eventuel transfert hors UE/EEE;
- la garantie de transfert lorsqu'applicable.

## Tableau

| Sous-traitant    | Service                                               | Catégories de données                                                                                                                                                                                                                                        | Localisation principale                                           | Transfert hors UE/EEE                                                            | Garantie                                               |
| :--------------- | :---------------------------------------------------- | :----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | :---------------------------------------------------------------- | :------------------------------------------------------------------------------- | :----------------------------------------------------- |
| **Scaleway**     | Hébergement (Compute, Database, Object Storage)       | Données de compte, fichiers, metadata, logs                                                                                                                                                                                                                  | France (fr-par)                                                   | Non                                                                              | N/A                                                    |
| **Scaleway**     | Generative APIs / inference IA                        | Données transmises volontairement aux fonctions IA, metadata de requêtes API, compteurs de tokens, paramètres utilisateur hors contenu des prompts; contenu complet de requête uniquement en cas d'incident, abus, erreur anormale ou investigation sécurité | France (Paris)                                                    | Non                                                                              | N/A                                                    |
| **Scaleway**     | Transactional Email (TEM) et Topics and Events        | Adresse et nom du destinataire, expéditeur, objet et corps du message, liens ou codes transactionnels, en-têtes et identifiants techniques, échéance d’envoi, statuts, erreurs, rebonds, spam et listes de suppression                                       | France (`fr-par`) et traitement TEM au sein de l’Union européenne | Non pour TEM au 12 août 2026, sous réserve des documents contractuels en vigueur | N/A                                                    |
| **Stripe**       | Paiement et Billing                                   | Coordonnées de facturation, TVA, identifiants transactionnels                                                                                                                                                                                                | USA (processing EU)                                               | Oui                                                                              | Clauses Contractuelles Types (SCC)                     |
| **PostHog**      | Product Analytics (Self-hosted possible)              | Events d'utilisation, metadata techniques                                                                                                                                                                                                                    | USA (Cloud)                                                       | Oui                                                                              | Clauses Contractuelles Types (SCC)                     |
| **Sentry**       | Observabilité (Error tracking)                        | Logs d'errors, metadata techniques (IP limitée)                                                                                                                                                                                                              | USA                                                               | Oui                                                                              | Clauses Contractuelles Types (SCC)                     |
| **Grafana Labs** | Observabilité Cloud (Metrics, Logs, Traces, Profiles) | Métriques techniques, traces redigées, logs redigés, profils CPU, labels techniques                                                                                                                                                                          | UE/USA selon stack                                                | Oui si région hors UE ou support global                                          | DPA Grafana Cloud + Clauses Contractuelles Types (SCC) |
| **Cloudflare**   | WAF, CDN, DNS, Rate Limiting                          | IP, logs réseau, requêtes HTTP                                                                                                                                                                                                                               | Global (Edge)                                                     | Oui                                                                              | Clauses Contractuelles Types (SCC)                     |

## Gouvernance

- Toute entree nouvelle ou modifiee doit etre revue avant mise en production.
- TEM est reserve aux communications transactionnelles. La prospection et les
  envois marketing doivent utiliser un canal distinct juridiquement et
  techniquement adapte.
- Le DPA, la liste des sous-traitants et la localisation TEM publies par
  Scaleway doivent etre archives puis verifies a chaque changement notifie.
- Les usages Scaleway Generative APIs doivent etre actives uniquement pour des
  finalites produit documentees. Le contenu client ne doit pas etre envoye a
  l'inference IA sans action explicite, consentement ou base contractuelle claire
  selon le cas d'usage.
- Grafana Cloud ne doit recevoir que des signaux passes par Alloy avec
  redaction/sampling; aucun credential Grafana Cloud ne doit etre configure dans
  les SDK applicatifs en production.
- Les claims marketing "heberge en Europe" ou "drive europeen" doivent etre relus a chaque changement de sous-traitant.
- Les clients doivent pouvoir acceder a cette liste dans une version publique a jour.
