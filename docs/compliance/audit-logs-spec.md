# Spécification technique des Audit Logs

## 1. Objectif

Les logs d'audit permettent aux administrateurs de workspace (Plan Enterprise) de tracer toutes les actions sensibles effectuées par les membres ou via l'API.

## 2. Structure d'un Audit Log

Chaque entrée d'audit contient au minimum :
- `timestamp` : Date et heure précises (ISO 8601).
- `actor_id` : ID de l'utilisateur ou de l'API Key à l'origine de l'action.
- `actor_type` : `USER`, `API_KEY`, `SYSTEM`.
- `action` : Code de l'action (ex: `file.delete`).
- `resource_id` : ID de l'objet impacté (fichier, membre, dossier).
- `workspace_id` : ID du workspace concerné.
- `ip_address` : Adresse IP source (anonymisée si nécessaire hors sécurité).
- `user_agent` : Client utilisé.
- `metadata` : JSON contenant les détails contextuels (sans données sensibles).

## 3. Événements tracés

| Catégorie | Événements |
| :--- | :--- |
| **Authentification** | `auth.login`, `auth.mfa.enabled`, `auth.password.changed` |
| **Workspace** | `workspace.member.invited`, `workspace.role.updated`, `workspace.billing.updated` |
| **Fichiers** | `file.created`, `file.deleted`, `file.shared`, `file.downloaded` |
| **Sécurité** | `api_key.created`, `api_key.revoked`, `permission.denied` |

## 4. Intégrité et Rétention

- **Immuabilité** : Les logs d'audit sont stockés dans une table PostgreSQL `audit_logs` avec des droits d'écriture uniquement (Insert). Aucune suppression (Delete) ou modification (Update) n'est autorisée via l'API applicative.
- **Rétention** : 
    - Plan Pro : 30 jours.
    - Plan Enterprise : 365 jours.
- **Archivage** : Au-delà de la rétention active, les logs sont archivés dans un bucket Object Storage sécurisé (froid) pendant 7 ans pour conformité légale.

## 5. Accès et Export

- Les administrateurs peuvent consulter les logs via le dashboard "Security & Audit".
- Un export CSV/JSON est disponible pour intégration dans un SIEM externe (Splunk, Datadog, etc.).

---
Dernière mise à jour : 2026-05-11
