# Liste des Sous-traitants

## Statut

Version à jour au 2026-05-11.

## Regles de publication

Pour chaque sous-traitant, documenter:

- le service fourni;
- les categories de donnees traitees;
- la localisation principale;
- un eventuel transfert hors UE/EEE;
- la garantie de transfert lorsqu'applicable.

## Tableau

| Sous-traitant | Service | Catégories de données | Localisation principale | Transfert hors UE/EEE | Garantie |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Scaleway** | Hébergement (Compute, Database, Object Storage) | Données de compte, fichiers, metadata, logs | France (fr-par) | Non | N/A |
| **Stripe** | Paiement et Billing | Coordonnées de facturation, TVA, identifiants transactionnels | USA (processing EU) | Oui | Clauses Contractuelles Types (SCC) |
| **PostHog** | Product Analytics (Self-hosted possible) | Events d'utilisation, metadata techniques | USA (Cloud) | Oui | Clauses Contractuelles Types (SCC) |
| **Sentry** | Observabilité (Error tracking) | Logs d'errors, metadata techniques (IP limitée) | USA | Oui | Clauses Contractuelles Types (SCC) |
| **Cloudflare** | WAF, CDN, DNS, Rate Limiting | IP, logs réseau, requêtes HTTP | Global (Edge) | Oui | Clauses Contractuelles Types (SCC) |

## Gouvernance

- Toute entree nouvelle ou modifiee doit etre revue avant mise en production.
- Les claims marketing "heberge en Europe" ou "drive europeen" doivent etre relus a chaque changement de sous-traitant.
- Les clients doivent pouvoir acceder a cette liste dans une version publique a jour.
