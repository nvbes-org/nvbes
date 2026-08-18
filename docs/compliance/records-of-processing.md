# Registre des Activités de Traitement (Art. 30 RGPD)

## 1. Gouvernance

- **Responsable du Registre** : DPO / Lead Compliance (nvbes Cloud SAS)
- **Dernière mise à jour** : 2026-08-12
- **Statut** : Version 1.6 - Projet à finaliser avant la mise en production de TEM.

## 2. Inventaire des Traitements

| Activité                                     | Rôle          | Finalité                                                     | Catégories de données                                                          | Base légale                   | Destinataires                     | Rétention                                                                                                      | Transfert hors-UE            |
| :------------------------------------------- | :------------ | :----------------------------------------------------------- | :----------------------------------------------------------------------------- | :---------------------------- | :-------------------------------- | :------------------------------------------------------------------------------------------------------------- | :--------------------------- |
| **Gestion des comptes**                      | Responsable   | Auth, Admin, Sécurité                                        | Email, Hash PWD, MFA, Logs sessions                                            | Contrat                       | Interne, Scaleway (Hébergement)   | 3 ans après inactivité                                                                                         | Non                          |
| **Communications transactionnelles Account** | Responsable   | Vérification, récupération, sécurité et exécution du contrat | Destinataire, objet, contenu transactionnel, identifiants et statuts de remise | Contrat                       | Scaleway TEM et Topics and Events | Charge utile : 30 j après statut terminal ; registre : 400 j ; bornes sans webhook et suppressions à finaliser | Non pour TEM au 12 août 2026 |
| **Hébergement Drive**                        | Sous-traitant | Stockage et partage de fichiers                              | Fichiers (binaires), Metadata                                                  | Contrat (Instructions client) | Scaleway (Object Storage)         | Durée du contrat + 30j (purge)                                                                                 | Non                          |
| **Facturation & TVA**                        | Responsable   | Paiement, Conformité fiscale                                 | Nom, Adresse, Métadonnées CB                                                   | Contrat / Loi                 | Stripe (Paiement)                 | 10 ans (Code de commerce)                                                                                      | Oui (USA via SCC)            |
| **Audit & Sécurité**                         | Responsable   | Détection intrusion, Imputabilité                            | Logs IP, User-Agent, Actions API                                               | Intérêt légitime              | Scaleway, Sentry (Debug)          | 1 an (Audit) / 90j (Debug)                                                                                     | Oui (USA via SCC - Sentry)   |
| **Support Technique**                        | Responsable   | Assistance utilisateur                                       | Email, Contenu des tickets                                                     | Contrat                       | Interne                           | 3 ans après clôture                                                                                            | Non                          |
| **Mesure d'audience**                        | Responsable   | Optimisation produit                                         | Pays, événements (pseudonymisé)                                                | Intérêt légitime              | Interne                           | 13 mois (recommandation CNIL)                                                                                  | Non                          |

## 3. Sous-traitants (Sub-processors)

| Nom            | Fonction                                       | Localisation                       | Garanties Transfert                                          |
| :------------- | :--------------------------------------------- | :--------------------------------- | :----------------------------------------------------------- |
| **Scaleway**   | Cloud Infrastructure (IaaS)                    | France (fr-par)                    | N/A (Souverain)                                              |
| **Scaleway**   | Transactional Email (TEM) et Topics and Events | France (fr-par) / Union européenne | N/A tant que TEM reste traité dans l’Union européenne        |
| **Stripe**     | Passerelle de paiement                         | Irlande / USA                      | Clauses Contractuelles Types (SCC)                           |
| **Sentry**     | Monitoring d'erreurs (Observability)           | USA                                | Clauses Contractuelles Types (SCC)                           |
| **Cloudflare** | Protection DNS / WAF                           | Global / EU                        | Clauses Contractuelles Types (SCC) + Data Localization Suite |

## 4. Mesures Techniques et Organisationnelles (TOMs)

Pour répondre aux exigences des **OIV/OES** et entreprises sensibles :

1.  **Isolation (Multi-tenancy)** : Séparation logique stricte des données par `workspace_id` au niveau SQL (Row Level Security envisagé).
2.  **Chiffrement** :
    - Flux : TLS 1.3 forcé, HSTS activé.
    - Repos : Chiffrement AES-256 via Scaleway Block Storage.
3.  **Hachage** : Utilisation d'Argon2id (mémoire-résistant) pour les secrets d'authentification.
4.  **Audit Immuable** : (En cours) Implémentation du chaînage de hash pour les logs d'audit sensibles.
5.  **Contrôle d'Accès** : MFA (TOTP, WebAuthn) disponible pour tous les comptes.

## Actions à finaliser (Backlog Compliance)

- [ ] Signer les DPA spécifiques avec les clients Enterprise.
- [ ] Valider la cartographie précise des flux Sentry (filtrage des PII au niveau SDK).
- [ ] Finaliser l'AIPD pour le traitement "Hébergement Drive".
- [ ] Borner la rétention des messages sans événement terminal et des listes de suppression email.
- [ ] Répercuter les demandes d’effacement et de déblocage auprès de Scaleway TEM.
