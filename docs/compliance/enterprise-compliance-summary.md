# nvbes : Conformité Enterprise & Secteurs Sensibles

Ce document synthétise l'état de préparation de nvbes pour l'onboarding de clients "Enterprise" (secteurs régulés, OIV, grandes entreprises), en gardant à l'esprit que le premier incrément ne couvre pas encore une suite enterprise complète.

## 1. Structure Légale et Gouvernance
- **Entité Légale** : nvbes Cloud SAS (Paris, France).
- **DPA (Data Processing Agreement)** : Finalisé et prêt pour signature. Définit nvbes comme sous-traitant (Data Processor).
- **Liste des Sous-traitants** : Nettoyée pour ne refléter que la stack réelle.
    - *Hébergement* : Scaleway (France).
    - *Paiement* : Stripe.
    - *Analytics/Product* : PostHog, Sentry.
    - *Réseau/Sécurité* : Cloudflare.

## 2. Engagements de Service (SLA)
Un document d'engagement de service a été créé pour garantir la transparence sur :
- **Disponibilité** : 99.9% pour le plan Enterprise.
- **Temps de Réponse Support** : < 2h pour les incidents critiques (P1).
- **Crédits de Service** : Mécanisme de compensation en cas de downtime.

## 3. Posture de Sécurité
- **VDP (Vulnerability Disclosure Policy)** : Politique publique pour l'accueil des signalements de chercheurs en sécurité.
- **Audit Logs** : Spécification technique pour une traçabilité complète des actions administratives (immuabilité, rétention 1 an).
- **Chiffrement** :
    - *Transit* : TLS 1.3 forcé via Cloudflare/API.
    - *Repos* : Chiffrement AES-256 (Scaleway Block Storage) et hachage Argon2id pour les mots de passe.

## 4. Conformité Privacy (RGPD)
- **Cookies** : Spécification conforme CNIL (Refus au premier niveau, traceurs essentiels uniquement par défaut).
- **Localisation des données** : Priorité au stockage souverain (Scaleway FR) pour les fichiers et la base de données.
- **Suppression des données** : Support du droit à l'oubli via les scripts de purge automatisés.

## Prochaines étapes suggérées (Maturité +)
1. **SSO / SAML** : Implémentation du support pour les IdP clients (Okta, Azure AD).
2. **SOC 2 Type I/II** : Préparation du contrôle interne pour certification future.
3. **Bring Your Own Key (BYOK)** : Permettre aux clients sensibles de gérer leurs propres clés de chiffrement pour leurs fichiers.

## Périmètre non encore finalisé

- federation enterprise complète;
- SCIM enterprise complet;
- attestation device enterprise complète;
- policy engine final.

---
*Dernière mise à jour : 2026-05-11*
*Documents de référence dans `docs/legal` et `docs/compliance`.*
