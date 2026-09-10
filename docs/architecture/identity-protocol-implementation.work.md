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

## Step-up WebAuthn transactionnel — 2026-09-10

`step_up::start` crée une cérémonie de cinq minutes liée à une session active,
avec les passkeys non révoquées de son principal. `step_up::finish` vérifie
l'assertion et l'UV contre l'état serveur, relit le credential sous verrou,
contrôle son compteur courant puis met à jour le Passkey, consomme le challenge
et accorde le step-up dans une seule transaction avec audit. La preuve récente
dure au plus dix minutes et ne dépasse pas l'expiration de la session.

La migration 0016 ajoute la révocation individuelle des credentials. Une clé
révoquée après le début de la cérémonie est refusée à la fin. Les anciens
credentials sans Passkey complet restent exclus. Les limites d'enrollment et
la présence d'un facteur existant portent désormais sur les clés non révoquées.

L'option `danger-credential-internals` de webauthn-rs est utilisée uniquement
pour lire le compteur typé du credential courant sous verrou, sans reconstruire
ni affaiblir la politique cryptographique. La vérification initiale utilise une
copie datant du début de cérémonie : ce second contrôle refuse une assertion
devenue obsolète après l'utilisation de la même clé dans une autre session.
La mise à jour reste faite par `Passkey::update_credential`.

Validation : compilation workspace sans avertissement et cible Nx
`identity-service:test:database` avec 117 tests de bibliothèque et 24 tests du
runtime réussis. Six tests PostgreSQL exercent les assertions signées, le rejeu,
la concurrence, la révocation, une autre session, l'expiration, une signature
altérée, les compteurs entre sessions et une panne d'écriture de session. La
panne annule aussi la mise à jour du credential et autorise la même preuve au retry.

Ce module interne n'est pas encore un parcours HTTP livré. Restent les adapters
Origin/CSRF/quotas, le login primaire passkey, la gestion des credentials,
la récupération et les parcours navigateur. Les lots A–D restent ouverts.

## Enrollment WebAuthn transactionnel — 2026-09-10

Les anciens helpers publics qui séparaient challenge aléatoire, état de
cérémonie, vérification et persistance ont été remplacés par `registration::start`
et `registration::finish`. Le challenge stocké est exactement celui généré par
webauthn-rs, avec son état complet, son principal et sa session. Une nouvelle
demande remplace la précédente pour cette session ; la durée est de cinq minutes.

Le début et la fin exigent une session active et une authentification récente.
Si le compte a déjà une passkey ou un MFA actif, une preuve récente WebAuthn ou
un step-up TOTP/WebAuthn est exigé. Le principal est verrouillé pour sérialiser
les ajouts depuis plusieurs sessions et borner les credentials à dix par compte.
La fin vérifie la réponse contre l'état serveur, puis écrit le Passkey complet,
consomme la cérémonie et audite dans une transaction unique.

La migration 0015 ajoute la liaison de session et le credential sérialisé.
Les anciennes lignes ne contenant qu'une clé publique ne sont pas transformées
en Passkey par supposition : elles nécessitent un nouvel enrollment. Le signal
discoverable reste inconnu (`NULL`) au lieu d'être affirmé par défaut. Le Passkey
complet est la source des compteurs, flags et règles de vérification ; les
anciennes colonnes isolées ne suffisent pas à authentifier.

Les tests PostgreSQL utilisent un authentificateur logiciel de la bibliothèque,
exclusivement en dépendance de développement. Ils produisent de vraies réponses
signées et vérifient une assertion après relecture du credential. Ils couvrent
origine incorrecte, absence d'UV, substitution de challenge, autre session,
expiration, révocation, rejeu, concurrence, facteur existant et panne d'écriture.
L'authentificateur de test simule l'UV ; il ne démontre aucune propriété matérielle.

Validation : `cargo check --workspace` sans avertissement ; cible Nx
`identity-service:test:database` avec 111 tests de bibliothèque et 24 tests du
runtime réussis, sur PostgreSQL isolé avec la migration 0015.

Cette tranche ne monte pas encore de routes WebAuthn : l'adaptateur HTTP doit
vérifier Origin et CSRF de session et appliquer les quotas avant ces opérations.
Restent le login/step-up transactionnel, les credentials découvrables, les
routes d'enrollment et de gestion, la récupération et les parcours navigateur.
Elle ne clôt donc pas le lot B ni les lots A–D.

## UserInfo et construction des endpoints — 2026-09-10

UserInfo dispose d'une audience dédiée, exige `openid` et ne retourne l'email
que si le scope est accordé et l'identifiant actuellement vérifié. Les jetons
Account/Billing et les ID tokens sont refusés. La session, le grant et la
politique client sont relus à chaque appel. Le traitement DPoP vérifie la clé,
la méthode, l'URL et `ath`, puis consomme la preuve dans la transaction de
lecture des claims. Un échec SQL annule cette consommation.

Les tests HTTP/PostgreSQL couvrent GET/POST, scopes, suspension, révocation,
réduction de politique, transport des credentials, headers dupliqués, preuves
invalides, rejeu concurrent et panne SQL avec nouvelle tentative de même preuve.
Les fixtures UserInfo utilisent des schémas privés pour isoler la panne injectée.
La construction des endpoints corrige aussi un défaut avec les issuers sans
slash final : discovery et cibles DPoP ont désormais un séparateur explicite,
sans modifier le claim `iss` configuré.

Validation : `cargo check --workspace` passe sans avertissement ; la cible Nx
`identity-service:test:database` passe avec 106 tests de bibliothèque et 24 tests
du runtime. Les parcours SDK/navigateur, WebAuthn et les autres exigences A–D
restent ouverts. Les échecs intermittents précédemment observés restent à
investiguer ; cette exécution verte ne démontre pas leur résolution.

## Échange de code atomique — 2026-09-10

POST `/oauth/token` utilise désormais `TokenService::exchange_code` : validation
cryptographique de la preuve DPoP sur l'URL canonique du service, puis une seule
transaction pour la consommation du code, le grant, le marqueur anti-rejeu,
la signature, le refresh éventuel et les audits. Un échec de signature ou
d'insertion du refresh annule l'ensemble. Aucun jeton n'est retourné avant commit.

Le rejeu authentifié d'un code consommé reste un résultat métier distinct :
la révocation et la consommation de sa nouvelle preuve sont committées avant
de retourner `invalid_grant`. Si la preuve elle-même a déjà été utilisée, la
transaction annule la tentative de révocation et son audit. Deux requêtes
identiques n'invalident donc pas le gagnant ; deux preuves fraîches sur le même
code déclenchent bien la révocation attendue.

Validation : `cargo check --workspace` sans avertissement ; la dernière cible
Nx `identity-service:test:database` passe avec 97 tests de bibliothèque, 24 tests
du runtime et 3 contrôles de base isolée. Les six nouveaux tests passent via le
routeur HTTP et PostgreSQL, dont une panne SQL après signature. Une première
exécution avait produit 12 échecs dans des tests de session/autorisation existants ;
la relance et l'exécution finale passent. Leur cause intermittente reste à
investiguer dans la validation globale, sans l'attribuer à l'environnement faute
de preuve. UserInfo et les parcours SDK/navigateur restent à terminer.

## Autorisation directe et PAR — 2026-09-10

Le décodage HTTP conserve les paires avant validation pour refuser les paramètres
dupliqués, y compris leurs noms encodés. `max_age` est converti explicitement en
entier et conserve la limite de 86400 secondes. La query directe et le corps PAR
sont bornés à 8192 octets et 32 paramètres. Un paramètre `request` non pris en
charge est refusé, sans repli silencieux vers un autre parcours.

PAR renvoie 201 avec une référence opaque. Sa rédemption accepte seulement
`client_id` et `request_uri`, sans paramètres capables de modifier la requête
enregistrée. La consommation du PAR, la création de l'unique interaction et sa
liaison navigateur/CSRF/session se font dans une même transaction PostgreSQL.
Un navigateur invalide ne consomme rien ; un échec de liaison annule aussi la
consommation et permet de réessayer. La création directe utilise la même frontière
transactionnelle. Une panne de consentement en mode silencieux renvoie 503,
sans redirection mensongère `login_required`.

Les tests HTTP/PostgreSQL couvrent les doublons, `max_age`, les substitutions,
le rejeu, la concurrence entre navigateurs, la panne injectée et le retour arrière.
Cette tranche ne valide pas encore un parcours SDK/navigateur jusqu'aux API
Account/Billing. UserInfo reste à terminer ; l'échange de code atomique est
documenté dans la tranche ci-dessus.

Validation : `cargo check --workspace` sans avertissement et
`identity-service:test:database` avec 91 tests de bibliothèque, 24 tests du runtime
et 3 contrôles de base isolée réussis. Aucun test graphique de navigateur n'est
inclus dans cette tranche.

## Quotas HTTP — 2026-09-10

Les routeurs OAuth exigent désormais un limiteur à clé stable ; le runtime exige
`NVBES_IDENTITY_RATE_LIMIT_KEY` lorsque le registre public est configuré et fournit
l'adresse TCP via `ConnectInfo`. Les en-têtes proxy ne sont pas utilisés.
Toutes les routes du protocole portent le quota de source avant parsing ; login
et TOTP ont un quota de source plus strict et un quota distinct par compte.
Les tentatives restent comptées après un refus métier. La migration 0014 étend
la contrainte des catégories à MFA sans effacer les compteurs existants.

HTTP 429 comporte `Retry-After` et `no-store`. Une panne ou un délai de quota
supérieur à deux secondes renvoie 503 ; il n'existe pas de fallback permissif.
Les tests couvrent les routes réelles, les headers proxy falsifiés, la clé,
la normalisation email/IPv6, la panne du stockage, l'expiration de fenêtre et
le partage du quota MFA entre sessions. Voir les limites et la configuration
d'exploitation dans [le cycle des jetons](identity-token-lifecycle.md#quotas-des-routes-http).
Les parcours navigateur, la topologie proxy, la charge et le budget restent
des gates de livraison ; aucune ouverture publique n'est effectuée.

Validation : `cargo check --workspace` sans avertissement ; la cible Nx
`identity-service:test:database` passe avec 85 tests de bibliothèque, 24 tests
du runtime et 3 contrôles de base isolée. Le test TCP utilise un vrai serveur
Axum sur loopback ; les autres tests HTTP traversent les routeurs et PostgreSQL.

## Calculs de mot de passe et annulation — 2026-09-10

La vérification Argon2, la vérification factice et le renouvellement du hash
conservent désormais leur permis de concurrence dans le travail bloquant.
L'annulation de la requête HTTP ne libère donc pas de capacité tant que le calcul
continue. Les tests pilotent explicitement un calcul en cours, annulent son
appelant puis vérifient que le permis reste occupé jusqu'à sa fin. Une panique
du calcul libère aussi le permis. Les quotas HTTP sont raccordés dans la tranche
documentée ci-dessus.

Validation : `cargo check --workspace` sans avertissement ;
`identity-service:test:database` passe avec 77 tests de bibliothèque, 24 tests
du runtime et 3 contrôles de base isolée. Les deux nouveaux tests de capacité
s'exécutent dans les deux cibles Rust qui compilent le module d'authentification.

## Login hébergé atomique — 2026-09-10

Le login contrôle l'interaction et son CSRF avant le calcul de mot de passe.
Argon2 et le calcul éventuel du nouveau hash s'exécutent sans transaction ouverte.
La transaction finale revérifie l'interaction, le registre, le statut du principal,
l'identifiant et le hash du mot de passe. Elle crée la session, écrit l'audit,
lie l'interaction et renouvelle le CSRF avant un unique commit.

Les tests HTTP/PostgreSQL vérifient l'absence de session orpheline sur mauvais
CSRF, les soumissions concurrentes et la permutation de session affichée. Un
trigger d'échec dans le schéma de test prouve le retour arrière de la session,
de l'audit et du CSRF lors d'un échec de liaison, puis la possibilité de réessayer.
Un autre test modifie les identifiants ou suspend le principal après vérification
du mot de passe et confirme que la création de session est refusée.

Validation : `cargo check --workspace` sans avertissement ; la cible Nx
`identity-service:test:database` passe avec 75 tests de bibliothèque, 22 tests
du runtime et 3 contrôles de base isolée. Ces preuves HTTP/PostgreSQL ne couvrent
pas encore les parcours de navigateur graphique.

## Sessions navigateur — 2026-09-10

Logout et step-up TOTP exigent une preuve CSRF liée par HMAC à la session,
au cookie navigateur et à l'origine. Le login et l'autorisation avec une session
active renvoient `session_csrf_token`, distinct du `csrf_token` de consentement.
Toutes les mutations hébergées passent par le middleware navigateur avant
lecture du JSON ; les deux mutations de session vérifient aussi le HMAC à cet
endroit. Le cookie navigateur est conservé entre autorisations concurrentes.

Le logout révoque et audite dans une transaction, sans audit dupliqué au rejeu.
Le step-up vérifie le statut actif du principal sous verrou. Les scénarios HTTP
avec PostgreSQL couvrent la preuve émise après login, les onglets concurrents,
le mauvais CSRF/origin/session, le refus avant parsing JSON, le code TOTP non
consommé lors d'un refus, le rejeu TOTP et la panne du stockage. Les fixtures
HTTP ont chacune un schéma isolé pour ne pas contaminer les tests de rotation
globale des clés MFA.

Validation : `cargo check --workspace` sans avertissement ;
`identity-service:test:database` passe avec 70 tests de bibliothèque, 22 tests
du runtime et 3 contrôles de base isolée. Les tests exécutent les routes Axum
et PostgreSQL ; aucun parcours de navigateur graphique n'est encore attesté.

Le logout intersites reste à terminer avant ouverture publique. Les limites
HTTP ont été raccordées dans la tranche documentée ci-dessus.

## Renouvellement et DPoP — 2026-09-10

Le refresh HTTP exige désormais le client enregistré et, pour un grant DPoP,
une preuve cryptographique de la même clé. Les rotations et rejeux de générations
différentes prennent les verrous dans le même ordre. La consommation de preuve,
la signature, la rotation et l'audit sont dans une transaction PostgreSQL.
Les headers DPoP dupliqués sont refusés sur le token endpoint ; les réponses de
jetons sont non stockables. Les erreurs de stockage deviennent HTTP 503.

Les tests actifs traversent le routeur token et PostgreSQL : mauvais client,
clé/méthode/URL erronées, preuve absente ou dupliquée, rejeu de preuve, rejeu du
refresh, concurrence, retour arrière après échec de signature, expiration,
révocation de session et suspension de principal. Ils ne prouvent pas encore
un parcours navigateur avec SDK et API de ressource.

Validation de cette tranche : `cargo check --workspace` sans avertissement ;
`identity-service:test:database` passe avec 62 tests de bibliothèque, 22 tests
du runtime et 3 tests du garde de base. Les migrations 0001–0013 sont appliquées
dans un conteneur PostgreSQL 17 dédié sur loopback, sans utiliser les bases
applicatives. Le passage des migrations WebAuthn ne valide pas ses cérémonies.

## Correction HTTP vérifiée au 2026-09-10

Les retours d'autorisation utilisent désormais HTTP 303, avec `Cache-Control:
no-store`, `Pragma: no-cache` et `Referrer-Policy: no-referrer`. Un test réseau
local suit les redirections après acceptation et refus : le callback reçoit
GET et aucun corps du POST initial. Les paramètres enregistrés et l'encodage
de `state` sont vérifiés. Ce test porte sur la construction et le suivi HTTP
de la réponse ; il ne prouve pas le parcours de consentement avec PostgreSQL.

Les handlers de session sont extraits pour maintenir le module HTTP sous 500
lignes. Les fonctions de récupération utilisées par le diagnostic interne
sont séparées de la bibliothèque OAuth, sans changement de logique ni nouvelle
route publique. `cargo check --workspace` passe sans avertissement.
La cible Nx `identity-service:test` passe : 36 tests de bibliothèque et 16 du
binaire, dont les deux nouveaux tests de redirection. Les tests PostgreSQL et
navigateur ne sont pas exécutés dans cette tranche.

Le tableau et les résultats ci-dessous restent un suivi historique partiel :
ils ne constituent pas une validation de bout en bout des lots A à D. Le
raccordement WebAuthn, DPoP dans les SDK/API, UserInfo et les contrôles
de session HTTP restent notamment à terminer et à tester.

## État historique au 2026-09-07, enrichi pendant l'implémentation

Les routes publiques sont montées sous configuration dans le runtime actif.
Leur présence ne vaut pas validation pour une ouverture publique. Les nouvelles
primitives sont dans la bibliothèque Identity, pas dans une application archivée.

| Exigence                                                                                 | État et preuve                                                                                                                                                                    |
| ---------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| A — clients publics explicitement enregistrés, redirections exactes, scopes par audience | Implémenté dans `identity.oauth.clients.rs` et `identity.oauth.request.rs` ; tests de refus redirection/scope/HTTP hors loopback                                                  |
| A — PKCE S256 et refus downgrade                                                         | Implémenté ; vecteur RFC 7636 et valeurs invalides testés                                                                                                                         |
| A — stockage des transactions et codes à usage unique                                    | Migration 0007 ; handles/codes hashés, sessions et principal contrôlés sous verrou                                                                                                |
| A — consommation concurrente, rejeu et révocation de grant                               | Tests PostgreSQL passent ; rejeu valide révoque le grant même après expiration du code                                                                                            |
| A — serveur HTTP autorisation/token, discovery, JWKS, UserInfo et ID tokens              | Discovery/JWKS/token/PAR/authorization, login, consentement et UserInfo sont montés sous configuration ; WebAuthn et UI complète restent à livrer                                 |
| A — consentement, cookies/CSRF/Origin/CORS et throttling                                 | Cookie navigateur et liaison transactionnelle sont montés sur GET authorization ; preuve CSRF/Origin, consentement et limites PostgreSQL restent actifs ; login UI reste à monter |
| A — audience/scopes cohérents Account/Billing, audit et erreurs publiques                | À terminer ; `billing:checkout` de PR 175 diverge du `billing:write` actuel                                                                                                       |
| B — passkeys, clés de sécurité et enrollment                                             | Persistance dédiée des credentials/challenges ajoutée en migration 0012 ; cérémonies webauthn-rs et classification d’assurance restent à implémenter                              |
| B — récupération, facteurs supplémentaires et politique opérateur                        | À implémenter ; ne pas confondre reset mot de passe et récupération MFA                                                                                                           |
| B — step-up frais / AMR exact / TOTP HTTP                                                | Méthode primaire et dates de preuve enregistrées ; endpoint POST step-up TOTP protégé par cookie/CSRF et rejeu de compteur ; WebAuthn et exigences AMR restantes                  |
| C — refresh rotation, familles, concurrence et détection de rejeu                        | Familles PostgreSQL hashées, rotation atomique et grant type HTTP `refresh_token` implémentés ; rejeu révoque famille/grant                                                       |
| C — introspection/révocation, rotation JWKS et logout intersites                         | Introspection du grant/session/principal, rotation de clés et logout HTTP avec révocation de session sont montés ; logout intersites/back-channel restent à livrer                |
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
