# Roadmap nvbes — V1 Ultra-Restreinte (Small Building)

## Autorité

Cette roadmap applique la [direction produit V1](product/nvbes-product-strategy.md).
Elle priorise une approche pragmatique de **"small building"** pour construire et
valider le cœur fonctionnel sans surcharge d'ingénierie.

---

## Objectif V1 : Service Développeur OAuth 2 & Compte

Délivrer une plateforme d'authentification et de gestion de compte centralisée
pour les développeurs utilisant les outils open-source Nvbes, avec :
- Une **API monolithique** (Auth, OAuth 2 PKCE, Consentement, Profil) ;
- Un **site web monolithique dev-first** (Login, Register, Consentement, Profil,
  zéro marketing) ;
- Un coût récurrent d'infrastructure strictement inférieur à **30 EUR TTC par mois**
  (cible opérationnelle à 20 EUR) ;
- Une garantie de **migration future vers des technologies open-source (OSS)
  auto-hébergées**, sans lock-in propriétaire.

---

## Jalons d'Exécution V1

### Jalon 1 — Cœur API Monolithique (OAuth 2, Sessions & Consentement)

- **Auth & Sessions** : Inscription, connexion, hachage sécurisé (Argon2),
  maintien de session.
- **Moteur OAuth 2.0** : Flux Authorization Code avec PKCE (RFC 7636), émission
  de tokens, rotation et révocation.
- **Gestion des consentements** : Écran de consentement, enregistrement et
  vérification des scopes autorisés par l'utilisateur pour une application.
- **Profil développeur** : Consultation du profil, liste des sessions actives
  et révocation.
- **Gestion des clients OAuth** : Configuration manuelle et interne (DB / config
  statique), sans portail en libre-service.

*Critère de sortie* : Tests d'intégration automatisés validant le cycle complet
Authorization Code + PKCE + Consentement en local.

### Jalon 2 — Web Monolithique Développeur (Dev-First)

- **Interface Authentification** : Pages d'inscription et de connexion claires,
  rapides et accessibles.
- **Interface Consentement** : Écran présentant l'application cliente et les
  scopes demandés pour acceptation ou refus.
- **Espace Profil** : Visualisation des informations du compte, statut et
  sessions actives.
- **Déploiement Frugal** : Hébergement statique optimisé (Cloudflare Pages,
  coût 0 € et bande passante illimitée).
- **Zéro front-end marketing** : Interface exclusivement dédiée aux
  développeurs authentifiés ou en cours d'autorisation.

*Critère de sortie* : Parcours utilisateur fluide dans le navigateur de
l'inscription jusqu'au consentement et à l'accès au profil.

### Jalon 3 — Validation Intégrée & Première Connexion d'Outil OSS

- **Validation End-to-End** : Raccordement d'un premier outil développeur
  open-source (CLI ou service web) via le protocole OAuth 2 standard.
- **Contrôles FinOps** : Vérification des dépenses réelles et de la descente à
  zéro / consommation ultra-réduite (< 20 Mo RAM pour l'API).
- **Sauvegarde et Restauration** : Procédure documentée de sauvegarde et de
  restauration de la base PostgreSQL.
- **Exploitation solo** : Runbook d'intervention manuelle pour l'opérateur.

*Critère de sortie* : L'outil développeur open-source obtient des jetons valides,
accède aux endpoints autorisés et gère le renouvellement sans erreur.

---

## Périmètre Gelé & Roadmap Post-V1

Les éléments suivants sont formellement exclus de la V1 et reportés aux phases
ultérieures :

1. **Monétisation & Billing (Stripe)** : Activation lors du lancement commercial
   du Cloud pour financer l'écosystème open-source.
2. **Trust/Risk Engine dédié** : Implémentation différée tant que les volumes
   d'abus ne justifient pas un composant d'analyse autonome.
3. **Workers asynchrones distribués** : Les tâches de fond légères restent
   traitées dans le binaire API unique.
4. **Gestion d'équipes et gouvernance B2B/Enterprise** : Reportées après
   l'adoption du socle par les premiers développeurs.
5. **Front-end marketing / vitrine** : Sera conçu lors de la promotion publique
   des outils.
6. **Migration vers l'Open Source auto-géré** : Bascule progressive des briques
   managées/serverless vers des instances PostgreSQL dédiées, Nginx/Caddy et
   outils auto-hébergés lorsque le trafic et le modèle économique le
   justifieront.
