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

| Exigence                                                                                 | État et preuve                                                                                                                   |
| ---------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| A — clients publics explicitement enregistrés, redirections exactes, scopes par audience | Implémenté dans `identity.oauth.clients.rs` et `identity.oauth.request.rs` ; tests de refus redirection/scope/HTTP hors loopback |
| A — PKCE S256 et refus downgrade                                                         | Implémenté ; vecteur RFC 7636 et valeurs invalides testés                                                                        |
| A — stockage des transactions et codes à usage unique                                    | Migration 0007 ; handles/codes hashés, sessions et principal contrôlés sous verrou                                               |
| A — consommation concurrente, rejeu et révocation de grant                               | Tests PostgreSQL passent ; rejeu valide révoque le grant même après expiration du code                                           |
| A — serveur HTTP autorisation/token, discovery, JWKS, UserInfo et ID tokens              | À implémenter ; métadonnées limitées à la couverture réelle                                                                      |
| A — login hébergé, consentement, cookies/CSRF/Origin/CORS et throttling                  | À implémenter ; les primitives synthétiques ne suffisent pas                                                                     |
| A — audience/scopes cohérents Account/Billing, audit et erreurs publiques                | À terminer ; `billing:checkout` de PR 175 diverge du `billing:write` actuel                                                      |
| B — passkeys, clés de sécurité et enrollment                                             | À implémenter avec webauthn-rs, sans attribution automatique AAL3                                                                |
| B — récupération, facteurs supplémentaires et politique opérateur                        | À implémenter ; ne pas confondre reset mot de passe et récupération MFA                                                          |
| B — step-up frais / AMR exact / TOTP HTTP                                                | À implémenter ; la politique héritée de PR 175 exige encore pwd et doit évoluer pour passkeys                                    |
| C — refresh rotation, familles, concurrence et détection de rejeu                        | À implémenter sur PostgreSQL, sans Redis obligatoire                                                                             |
| C — introspection/révocation, rotation JWKS et logout intersites                         | À implémenter ; tokens doivent consulter le grant révoqué selon la politique explicite                                           |
| D — PAR HTTP et consommation atomique                                                    | Consommation PostgreSQL testée ; exposition HTTP restante                                                                        |
| D — DPoP AS + SDK + serveurs de ressources                                               | À implémenter ; la validation actuelle de jkt ne vérifie pas encore une preuve DPoP                                              |
| D — délégation machine si nécessaire                                                     | À statuer selon façade sécurité ; pas de reprise du Token Exchange Cloud                                                         |
| Gates — contrats, vrais parcours, caches, panne, migration et budget                     | À compléter avant clôture ; aucune ressource payante ajoutée                                                                     |

## Prochaine tranche

1. Relier l'émission des tokens au grant actif, avec ID token OIDC, auth_time et
   distinction des audiences. Ne pas fournir aux appelants un moyen de déclarer
   eux-mêmes AMR ou DPoP sans preuve validée.
2. Implémenter la session navigateur et le login hébergé avec consentement lié à
   la transaction, cookies hôte, CSRF, Origin et limites PostgreSQL bornées.
3. Monter les routes Code/PKCE et OIDC complètes, tester les erreurs de protocole
   et documenter leurs contrats. Puis poursuivre B, C et D sans réduire le scope.

Les tables de transactions doivent avoir un nettoyage borné avant exposition.
La fraîcheur, prompt=login/none, le refus utilisateur, le renouvellement et la
politique de déconnexion ne sont pas couverts par les seuls tests de stockage.

## Validation effectuée

- `cargo check --workspace` : succès dans le worktree isolé.
- Cible Nx `identity-service:test:database` : 10 tests de bibliothèque OAuth et
  28 tests du binaire réussis, plus 3 tests du garde de base de données.
- Base dédiée `nvbes_identity_test_protocol`, PostgreSQL local sur port 15432 ;
  migrations 0001–0007 appliquées. Aucun test sur les bases applicatives.
- Les 10 tests comprennent 6 validations pures et 4 scénarios PostgreSQL, avec
  courses concurrentes pour PAR et échange du code.
- Cible Nx `identity-service:test:contract` : 11 tests réussis ; ce contrat
  confirme pour l'instant que le runtime public reste fermé.
- Les routes publiques, facteurs WebAuthn, refresh et DPoP complets restent à
  tester après leur implémentation : le vert actuel ne prouve pas les lots A–D.

Pour exécuter Nx dans ce worktree, `node_modules` est un lien local vers les
dépendances du checkout principal ; `pnpm_config_verify_deps_before_run=false`
évite que pnpm tente de réinstaller ce répertoire partagé. Il ne désactive pas
les tests ni les hooks. Les dépendances Rust sont compilées dans le target propre
au worktree. Ne pas committer le lien node_modules ni le document Stripe local.
