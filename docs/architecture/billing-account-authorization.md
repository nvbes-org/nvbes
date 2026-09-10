# Autorisation des comptes facturés

Identity établit le principal, l'activité de sa session et les scopes. Account
décide de l'accès au compte facturé. Billing applique ces deux contrôles avant
toute lecture, recherche d'idempotence, création de customer ou appel Stripe.
Un scope `billing:read` ou `billing:checkout` ne confère aucun rôle d'équipe.

## Politique V1

| Cible       | Condition Account                                                               |
| ----------- | ------------------------------------------------------------------------------- |
| `principal` | ID demandé égal au principal authentifié, profil existant et actif              |
| `team`      | Équipe active, principal propriétaire déclaré, membership `owner`, profil actif |

Un membre ordinaire, un tiers, une cible absente, un profil en fermeture ou une
équipe fermée est refusé. Une incohérence entre propriétaire déclaré et membership
est également refusée. Cette politique s'applique à overview, portal et checkout ;
leurs scopes restent respectivement `billing:read`, `billing:read` et
`billing:checkout`. Le premier accès à Account crée le profil personnel ;
l'autorisation Billing ne crée pas de profil implicitement.

## Routes Billing

Les routes typées sont `/accounts/{account_type}/{id}/billing/overview` (GET),
`/accounts/{account_type}/{id}/billing/portal` (POST) et
`/accounts/{account_type}/{id}/billing/checkout` (POST).
`account_type` vaut exactement `principal` ou `team`.

Les routes existantes `/workspaces/{id}/billing/*` restent des alias **team**.
Le champ optionnel `account_type` du corps checkout doit correspondre à la route,
sinon la requête est refusée. Les lectures SQL sélectionnent l'ID **et** le type,
comme les contraintes uniques du schéma Billing. Aucun fallback entre types.

## Contrat interne Account

`POST /internal/v1/billing/authorize` reçoit les UUID `principal_id`, `account_id`
et l'enum `account_type`. Il retourne ces mêmes champs et le booléen `allowed`.
Aucun profil, nom, email ou détail d'appartenance n'est renvoyé.

Le endpoint exige un bearer de service dédié, distinct des jetons utilisateurs
et des secrets d'introspection Identity. La variable
`NVBES_ACCOUNT_BILLING_AUTHORIZATION_SECRET` contient 32 octets aléatoires encodés
en 64 caractères hexadécimaux minuscules, identiques dans Account et Billing.
Account vérifie un seul header Authorization et compare les empreintes SHA-256
en temps constant. Sans secret configuré, la route interne est absente.

Billing configure aussi `NVBES_BILLING_ACCOUNT_ORIGIN` : origine HTTPS, ou HTTP
loopback en développement. La paire origine/secret doit être complète. Sans
les deux valeurs, les opérations utilisateur Billing retournent 503 ; les
webhooks gardent leur authentification Stripe indépendante. Aucun credential
utilisateur n'est transmis à Account : Billing envoie le principal qu'il vient
de vérifier auprès d'Identity. Le secret de service représente donc cette
frontière de confiance et reste exclusivement serveur.

Account borne à 16 requêtes simultanées, deux secondes et 1 Kio de corps.
Billing borne à 16 consultations, 2,5 secondes et 1 Kio de réponse ; il refuse
redirections, proxies implicites, réponse ambiguë et substitution de cible.
Un refus métier donne 403 `account_access_denied`. Une panne, une réponse
incohérente ou une saturation donne 503 `account_unavailable`. Aucun retry,
cache positif ou autorisation locale de secours n'est utilisé.

La décision reflète l'état Account lu pour cette requête ; une requête déjà
autorisée peut terminer pendant une modification concurrente. Il ne s'agit pas
d'une transaction distribuée entre Account et Stripe. Les requêtes suivantes
relisent les droits. Le mécanisme n'ajoute aucun service, base, abonnement ou
traitement en arrière-plan. Son coût et les démarrages à froid restent à mesurer
dans les gates de charge/FinOps avant exposition publique.

## Validation

Les tests Account avec PostgreSQL couvrent propriétaire/membre/tiers, type,
fermeture, incohérence de rôle, authentification du service, corps borné et
store indisponible. Les tests du client Billing couvrent refus, substitution,
réponses malformées, saturation et panne.

`pnpm nx run identity-service:test:resource-runtimes` exerce les trois vrais
binaires avec Code/PKCE et données éphémères : accès personnel et équipe,
refus avant effets métier, changement de rôle avec le même JWT, fermeture,
panne/reprise d'Account et d'Identity, et révocation de session. Les mutations
autorisées utilisent uniquement le provider dummy local de Billing. Aucun
appel Stripe réel, déploiement ou ouverture publique n'est réalisé.
