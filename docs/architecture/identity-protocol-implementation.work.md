# Exécution des lots Identity A à D

## Objectif et périmètre conservé

Implémenter intégralement les lots A à D de
[l'inventaire](identity-archive-mechanisms-review.md), dans les services actifs,
avec migrations réelles et tests adversariaux. Ce suivi ne remplace pas les
exigences du document source et n'autorise aucune ouverture partielle en public.

Branche : `codex/frontend-platform-preparation`, PR 210. Worktree isolé :
`/Users/shayn/Development/nvbes-identity-protocol` (le checkout principal est
utilisé en parallèle). La branche de PR Identity 175, commit `890e0b7e`, a été
intégrée localement comme dépendance par merge signé `ce7adc9b`. Aucune PR n'a
été fusionnée dans main et aucune infrastructure n'a été déployée.

## État vérifié au 2026-09-07

Le protocole public n'est **pas encore monté**. Les nouvelles primitives sont
dans la bibliothèque du package Identity, pas dans une application archivée.

| Exigence                                                                                 | État et preuve                                                                                                                                                                    |
| ---------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| A — clients publics explicitement enregistrés, redirections exactes, scopes par audience | Implémenté dans `identity.oauth.clients.rs` et `identity.oauth.request.rs` ; tests de refus redirection/scope/HTTP hors loopback                                                  |
| A — PKCE S256 et refus downgrade                                                         | Implémenté ; vecteur RFC 7636 et valeurs invalides testés                                                                                                                         |
| A — stockage des transactions et codes à usage unique                                    | Migration 0007 ; handles/codes hashés, sessions et principal contrôlés sous verrou                                                                                                |
| A — consommation concurrente, rejeu et révocation de grant                               | Tests PostgreSQL passent ; rejeu valide révoque le grant même après expiration du code                                                                                            |
| A — serveur HTTP autorisation/token, discovery, JWKS, UserInfo et ID tokens              | Discovery/JWKS/token/PAR et démarrage GET authorization sont montés sous configuration ; UserInfo, login hébergé et écran de consentement restent à livrer                        |
| A — consentement, cookies/CSRF/Origin/CORS et throttling                                 | Cookie navigateur et liaison transactionnelle sont montés sur GET authorization ; preuve CSRF/Origin, consentement et limites PostgreSQL restent actifs ; login UI reste à monter |
| A — audience/scopes cohérents Account/Billing, audit et erreurs publiques                | À terminer ; `billing:checkout` de PR 175 diverge du `billing:write` actuel                                                                                                       |
| B — passkeys, clés de sécurité et enrollment                                             | À implémenter avec webauthn-rs, sans attribution automatique AAL3                                                                                                                 |
| B — récupération, facteurs supplémentaires et politique opérateur                        | À implémenter ; ne pas confondre reset mot de passe et récupération MFA                                                                                                           |
| B — step-up frais / AMR exact / TOTP HTTP                                                | Méthode primaire et dates de preuve enregistrées ; AMR issu du snapshot d'autorisation, passkeys sans pwd possibles ; HTTP restant                                                |
| C — refresh rotation, familles, concurrence et détection de rejeu                        | Familles PostgreSQL hashées et rotation atomique implémentées ; rejeu révoque famille/grant ; routes HTTP et logout restent à monter                                              |
| C — introspection/révocation, rotation JWKS et logout intersites                         | Introspection du grant/session/principal et registre courant testée ; rotation de clés avec échéances testée ; HTTP/logout restants                                               |
| D — PAR HTTP et consommation atomique                                                    | Endpoint POST `/oauth/par` validant puis stockant une requête opaque ; consommation PostgreSQL atomique prête pour le raccordement authorization                                  |
| D — DPoP AS + SDK + serveurs de ressources                                               | Preuve ES256, `htm`/`htu`/iat et thumbprint vérifiés ; consommation anti-rejeu PostgreSQL ajoutée ; raccordement route token et resource servers restant                          |
| D — délégation machine si nécessaire                                                     | À statuer selon façade sécurité ; pas de reprise du Token Exchange Cloud                                                                                                          |
| Gates — contrats, vrais parcours, caches, panne, migration et budget                     | À compléter avant clôture ; aucune ressource payante ajoutée                                                                                                                      |

## Prochaine tranche

1. Monter le login hébergé et les routes Code/PKCE/OIDC sur les primitives de
   navigateur et de consentement désormais disponibles ; tester les erreurs de protocole
   et documenter leurs contrats. Puis poursuivre B, C et D sans réduire le scope.
2. UserInfo doit disposer d'une audience explicitement autorisée : ne pas accepter
   arbitrairement un access token destiné exclusivement à Account ou Billing.
   La prise en charge des scopes OIDC/profile/email doit correspondre aux claims
   réellement servis. Le jkt doit devenir une preuve DPoP vérifiée, pas une chaîne
   déclarée par l'appelant.

Les tables de transactions disposent d'un nettoyage borné avant exposition. Les
timestamps de fraîcheur sont enregistrés et testés, mais leur contrôle dans
Account/Billing reste à implémenter. `prompt=login`/`prompt=none`, le refus
utilisateur et la rotation de la preuve CSRF après login sont couverts par les
primitives ; le renouvellement et la politique de déconnexion restent à livrer.

L'émission a été déplacée dans la bibliothèque du package, sans seconde pile de
signature. Le diagnostic synthétique traverse désormais le vrai grant OAuth.
La migration 0008 ajoute les preuves primaires, les dates de step-up, le snapshot
d'autorisation et la consommation unique de l'émission. La migration 0009 ajoute
la liaison navigateur/CSRF/session des interactions, les consentements exacts et
les buckets de limite fixes. La migration 0010 ajoute les familles de refresh
hashées. Voir le
[cycle des jetons et la procédure de rotation](identity-token-lifecycle.md).

## Validation effectuée

- `cargo check --workspace` : succès dans le worktree isolé.
- Cible Nx `identity-service:test:database` : 40 tests de bibliothèque OAuth,
  navigateur, consentement, limite et tokens, 22 tests du binaire, plus 3 tests
  du garde de base de données réussis.
- Base dédiée `nvbes_identity_test_protocol`, PostgreSQL local sur port 15433 ;
  migrations 0001–0011 appliquées. Aucun test sur les bases applicatives.
- Tests PostgreSQL de concurrence PAR, échange de code et émission ; rejeu
  invalidant un jeton déjà émis ; suspension avant signature ; retrait du client
  et preuves MFA capturées avant une modification ultérieure de la session.
- Tests cryptographiques d'audiences/types distincts, at_hash/nonce/auth_time,
  claims hostiles, clés faibles/non appariées, rotation avec échéance et AMR exact.
- Tests des frontières navigateur : cookie de développement explicitement distinct,
  cookie concurrent ou header dupliqué refusés, origine exacte, métadonnées Fetch,
  CSRF et type JSON requis avant toute mutation. Les consentements sont liés à
  l'interaction, à la session affichée et à une politique exacte ; concurrence,
  session révoquée, registre modifié, consentement expiré et mode silencieux sont
  couverts. Les limites utilisent 4096 buckets HMAC par catégorie et leur
  nettoyage ne supprime pas les preuves nécessaires au rejet de rejeu.
- Tests de rotation refresh sur PostgreSQL : un seul secret actif à la fois,
  rotation vers une nouvelle valeur opaque, rejeu de l'ancienne valeur et
  invalidation de la famille et du grant.
- Tests DPoP : signature ES256, méthode/URL incorrectes et consommation unique
  du `jti` pour une même clé sur la base PostgreSQL.
- Cible Nx `identity-service:test:contract` : 11 tests réussis ; ce contrat
  confirme pour l'instant que le runtime public reste fermé.
- Les routes publiques, facteurs WebAuthn, refresh et DPoP complets restent à
  tester après leur implémentation : le vert actuel ne prouve pas les lots A–D.

Pour exécuter Nx dans ce worktree, `node_modules` est un lien local vers les
dépendances du checkout principal ; `pnpm_config_verify_deps_before_run=false`
évite que pnpm tente de réinstaller ce répertoire partagé. Il ne désactive pas
les tests ni les hooks. Les dépendances Rust sont compilées dans le target propre
au worktree. Ne pas committer le lien node_modules ni le document Stripe local.
