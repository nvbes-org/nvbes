# Direction produit nvbes V1

## Statut et autorité

Ce document est la source de vérité produit pour la V1 actuelle. En cas de
contradiction, il prévaut sur tous les anciens PRD, blueprints, plans
d'exécution et documents marketing. Les ADR conservent leur valeur historique,
mais ne peuvent pas élargir le périmètre produit actif.

---

## 1. Mission globale de Nvbes

**Nvbes est une entité destinée à offrir des outils et des logiciels aux développeurs, le tout en open-source, financé par son futur cloud.**

Le cloud servira de modèle économique pour soutenir et pérenniser l'écosystème
d'outils open-source.

---

## 2. Décision V1 : Approche "Small Building" & V1 Ultra-Restreinte

Le périmètre initial du projet étant trop vaste pour démarrer efficacement, la V1
adopte une démarche stricte de **"small building"** : construire une tranche
verticale ultra-restreinte, hautement fonctionnelle, sans friction
opérationnelle ni surcharge architecturale prématurée.

Dans cette V1 ultra-restreinte, Nvbes sert exclusivement en tant que **service
d'authentification et de compte pour les développeurs** utilisant ses outils :

- **Configuration interne et manuelle** : les applications clientes OAuth 2 sont
  déclarées et gérées manuellement (base de données ou configuration statique),
  sans portail d'enregistrement d'apps en libre-service.
- **Une API monolithique** (`apps/api`) :
  1. Création de compte, Connexion et maintien de session, le tout en **OAuth 2**
     (standard RFC 6749 avec PKCE) ;
  2. Gestion des consentements (écran d'approbation et persistance des scopes
     accordés par l'utilisateur) ;
  3. Profil utilisateur de base et consultation des sessions actives.
- **Un site web monolithique** (`apps/web` / interfaces unifiées) :
  1. Pages d'inscription et de connexion en OAuth 2 ;
  2. Écran d'accord de consentement aux applications clientes ;
  3. Page de profil (informations du compte, déconnexion / révocation de session).
  4. Interface strictement dev-first : **aucun front-end marketing en V1**.

---

## 3. Les Deux Principes Directeurs Majeurs

Toutes les décisions techniques et d'infrastructure de la V1 reposent sur deux
piliers non négociables :

1. **Rentabilité maximale & technologies les moins chères du marché** :
   - Utiliser au maximum les technologies gratuites, les plans free tier et les
     runtimes à empreinte ultra-réduite (Rust/Axum consommant moins de 20 Mo de
     RAM, Cloudflare Pages gratuit pour le front statique avec bande passante
     illimitée, base PostgreSQL managée low-cost ou free tier).
   - Coût récurrent d'infrastructure strictement plafonné à **30 EUR TTC par mois**
     (cible opérationnelle à 20 EUR).

2. **Migration vers l'Open Source auto-géré dans le futur** :
   - Aucun enfermement propriétaire (zéro lock-in) : chaque technologie ou
     fournisseur utilisé doit pouvoir être remplacé à tout moment par son
     équivalent open-source auto-hébergé (PostgreSQL natif sur VPS/dédié, reverse
     proxy Nginx ou Caddy, briques de messagerie SMTP standard).
   - Les frontières logicielles et les contrats d'API doivent être standards
     (RFC OAuth2, requêtes HTTP/JSON standard).

---

## 4. Public cible et posture réglementaire

- **Public cible** : Les **développeurs** utilisant les outils open-source Nvbes.
- **B2B et Enterprise** : Hors périmètre V1. Aucun contrat d'entreprise, ni
  SLA garanti, ni SSO d'entreprise (SAML/SCIM), ni gouvernance complexe.
- **Souveraineté et conformité avancée** : **Non exigées pour la V1**, mais
  gardées en tête pour des migrations ultérieures dès lors que le produit aura
  trouvé son adéquation marché. Les obligations légales élémentaires (RGPD de
  base sur la conservation des comptes) sont appliquées sans surcoût
  opérationnel.

---

## 5. Périmètre V1 : Inclus vs Gelé

### Périmètre actif inclus
- Moteur d'authentification et serveur d'autorisation OAuth 2.0 (PKCE, sessions) ;
- Gestion du consentement utilisateur (scopes demandés par les outils dev) ;
- API et Web monolithiques dédiés aux développeurs ;
- Gestion manuelle et interne des applications clientes OAuth 2 ;
- Profil utilisateur minimal (consultation, révocation de session, modification de base) ;
- Suivi FinOps strict (coût < 30 EUR TTC/mois).

### Périmètre gelé / reporté post-V1
- ❌ **Billing (Stripe)** : Gelé tant que les services payants du cloud ne sont
  pas prêts à la commercialisation ;
- ❌ **Trust/Risk engine séparé** : Remplacé par des contrôles d'accès et rate
  limits simples en mémoire ;
- ❌ **Multi-workers d'arrière-plan** : Les besoins asynchrones légers sont
  exécutés directement dans le runtime API (Tokio tasks) ;
- ❌ **Gestion d'équipes et organisations complexes** : Reportée ;
- ❌ **Front-end marketing / vitrine** : Zéro investissement avant validation des
  outils développeurs.

---

## 6. Exploitation et Opérateur Solo

Un seul opérateur administre Nvbes en V1. Par défaut, les interventions, le
support et la configuration des applications clientes sont traités
manuellement.

Une automatisation n'est recevable que si elle :
- répond à un besoin réel mesuré ;
- maintient le coût total sous 30 EUR TTC/mois ;
- est réversible et remplaçable par une solution open-source auto-gérée ;
- ne prend pas de décision critique de façon opaque.

---

## 7. Qualité et "Zéro dette technique livrée"

Même en approche ultra-restreinte, **aucune dette technique ou structurelle
connue n'est acceptée dans le périmètre livré**. 

Le code doit être propre, typé strictement, testé de bout en bout et exempt de
rustines temporaires. Si une conception s'avère inadaptée, le redesign complet
est privilégié.
