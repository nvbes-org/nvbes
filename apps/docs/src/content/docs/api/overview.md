---
title: Standards & Conventions d'API
description: Normes de conception des APIs REST, authentification DPoP, pagination et gestion des erreurs.
---

## Authentification & Sécurité

Toutes les requêtes vers les APIs de production doivent inclure :
- **Authorization** : `DPoP <access_token>` ou `Bearer <jwt>`
- **DPoP Proof** : En-tête `DPoP: <jwt_proof>` certifiant la clé privée du client HTTP
- **X-Request-Id** : Identifiant UUID v4 unique pour le traçage distribué

## Format Normalisé des Erreurs

Les erreurs d'API suivent la structure normalisée suivante :

```json
{
  "error": {
    "code": "INVALID_INPUT",
    "message": "Le champ email est requis et doit être valide.",
    "details": {
      "field": "email"
    }
  }
}
```
