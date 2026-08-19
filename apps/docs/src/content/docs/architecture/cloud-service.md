---
title: Service Cloud & Gestion des Fichiers
description: Architecture du service Cloud/Drive, quotas de stockage, chiffrement et upload/download multipart.
---

## 1. Rôle Architectural

API de stockage objet, gestion des espaces partagés, métadonnées de fichiers et contrôle d accès fin.

- **Domaine métier** : `cloud`
- **Mécanisme d'authentification** : Bearer DPoP JWT (émis par Identity)
- **Scopes requis** : `drive:read`, `drive:write`, `drive:share`, `drive:admin`

---

## 2. Invariants & Règles Métier

Ces règles constituent les garanties fondamentales du service :

- **Règle** : Tout upload direct vers le stockage objet nécessite une URL pré-signée validée par cloud-service.
- **Règle** : Les quotas d espace disque sont vérifiés de manière atomique avant l émission des tokens d upload.
- **Règle** : Le partage de documents applique un modèle de permissions immuable avec révocation instantanée.

---

## 3. Flux Principal (Diagramme de Séquence)

```mermaid
sequenceDiagram
    autonumber
    actor User as "Client Drive Web"
    participant GW as "gateway-cloud"
    participant Cloud as "cloud-service"
    participant Storage as "Object Storage S3"
    participant DB as "PostgreSQL (Cloud DB)"

    User->>GW: POST /api/v1/files/upload-ticket
    GW->>Cloud: Verification du quota et des droits
    Cloud->>DB: Verifie l espace disque disponible
    Cloud->>Storage: Genere l URL pre-signee S3
    Cloud-->>User: 200 OK (URL pre-signee S3 + Token)
    User->>Storage: PUT Direct vers S3 avec le binaire
    Storage-->>User: 200 OK (Upload termine)
    User->>Cloud: POST /api/v1/files/confirm-upload
    Cloud->>DB: Enregistre les metadonnees du fichier
```

---

## 4. Matrice des Erreurs & Stratégies de Résilience

| Type d'Erreur | Code HTTP | Stratégie de Récupération |
| :--- | :--- | :--- |
| `QuotaExceeded` | `402` | Inviter l utilisateur à mettre à niveau son abonnement de stockage. |
| `UnauthorizedAccess` | `403` | Refuser l accès, journaliser l événement dans l audit append-only. |
| `FileNotFound` | `404` | Vérifier la validité de l identifiant ou si le fichier a été purgé. |
