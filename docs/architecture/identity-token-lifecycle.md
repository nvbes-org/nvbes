# Jetons du service Identity actif

Ce document décrit la bibliothèque et les routes OAuth montées sous configuration
dans le service actif. Account/Billing consomment l'introspection ; les parcours
multisites complets et les autres capacités restent à terminer dans les
[lots A à D](identity-protocol-implementation.work.md).

## Émission et révocation

L'échange d'un code produit une capacité d'émission liée au grant PostgreSQL.
Sur l'endpoint HTTP, `TokenService::exchange_code` conserve dans une transaction
la consommation du code et de la preuve DPoP, le grant, les signatures, le refresh
éventuel et les audits. Un échec annule ces changements et autorise une nouvelle
tentative avec le même code et la même preuve tant qu'ils restent valides.
La réponse n'est envoyée qu'après commit ; cela ne garantit pas sa réception
par le client en cas de rupture réseau après commit.

Un code rejoué avec ses preuves valides révoque son grant. La révocation et le
marqueur d'une nouvelle preuve DPoP sont committés avant l'erreur `invalid_grant`.
Le rejeu d'une preuve DPoP déjà consommée provoque au contraire un retour arrière,
sans invalider les jetons du premier appel. La cryptographie utilise l'issuer
canonique détenu par le service, sans URL de token indépendante dans le routeur.

L'émission verrouille ce grant, la session et le principal, puis relit le registre
des clients. Une seule réponse peut être émise, même avec deux appels concurrents.
Une session expirée/révoquée, un principal suspendu, un client retiré ou une
politique de scopes rétrécie font échouer l'émission. Les diagnostics opérateur
empruntent ce même parcours et ne disposent plus d'une API de signature directe
à partir d'un identifiant de session.

L'access token est un JWT RS256 `typ=at+jwt`, avec une audience de ressource
exacte, `client_id`, `grant_id`, `sid` et les seuls scopes de cette API. L'ID token
est un JWT `typ=JWT` dont l'audience est le client OIDC ; il contient `nonce`,
`auth_time`, `amr` et `at_hash`. Un ID token est refusé comme access token. Les
contrats sont [access-token.v1](../../contracts/identity/access-token.v1.schema.json)
et [id-token.v1](../../contracts/identity/id-token.v1.schema.json). Cette séparation
reprend les audiences et types définis par [OIDC Core](https://openid.net/specs/openid-connect-core-1_0.html#IDToken)
et le [profil JWT d'access token](https://www.rfc-editor.org/rfc/rfc9068.html).

### Ressource UserInfo

GET et POST `/oauth/userinfo` acceptent uniquement un access token dans
`Authorization`. Les query strings, corps non vides et headers Authorization ou
DPoP dupliqués sont refusés. Les réponses portent `Cache-Control: no-store`.

Pour utiliser cette ressource, ajouter `nvbes-identity-userinfo` à
`NVBES_IDENTITY_TOKEN_AUDIENCES` et enregistrer dans les ressources du client
l'URL exacte de l'endpoint UserInfo, avec cette audience et les scopes autorisés
parmi `openid`, `profile`, `email`. `openid` est obligatoire. L'autorisation doit
demander cette ressource explicitement ; un jeton Account ou Billing ne donne
pas accès à UserInfo. `offline_access`, lorsqu'autorisé pour le client, contrôle
l'émission du refresh et n'est jamais inclus dans les scopes de l'access token.

La réponse contient `sub`. Avec `email`, elle peut ajouter l'identifiant email
actuellement vérifié et `email_verified: true`. Aucun profil Account n'est
inventé ni exposé : `profile` n'ajoute actuellement aucun claim. Le grant, la
session, le principal et la politique courante du client sont vérifiés à chaque
appel. Une révocation ou un retrait des scopes interdit donc l'accès immédiatement.

Un jeton lié à DPoP exige le schéma `Authorization: DPoP` et une preuve liée à
sa clé, la méthode HTTP, l'URL canonique et son hash `ath`. La consommation
anti-rejeu et la lecture des claims partagent une transaction : une panne SQL
renvoie 503 et ne consomme pas la preuve. Aucun repli Bearer n'est accepté.

L'issuer conserve exactement sa valeur configurée pour `iss`. Les URL des
endpoints sont construites avec un séparateur `/`, même sans slash final dans
l'issuer. Les SDK et parcours navigateur doivent encore intégrer la demande
explicite de cette ressource ; ces tests serveur ne prouvent pas leur livraison.

### Durée de vie et introspection

La durée de vie est plafonnée à 900 secondes et à l'expiration de la session.
La vérification cryptographique seule ne lit pas l'état PostgreSQL. L'introspection
relit le grant, la session, le principal et le registre courant : le rejeu valide
d'un code invalide donc aussi les jetons déjà émis. En cas de panne du stockage,
l'introspection renvoie une erreur, jamais un résultat actif par défaut. Les
consommateurs exigeant une autorisation actuelle devront refuser l'opération si
ce contrôle est indisponible. Leur intégration fait encore partie des lots.

### Introspection HTTP pour les API

POST `/oauth/introspect` reçoit un formulaire `token` et éventuellement
`token_type_hint` (ignoré). Les paramètres dupliqués, inconnus et les paramètres
en query sont refusés. L'API suit le contrat requête/réponse de la
[RFC 7662](https://www.rfc-editor.org/rfc/rfc7662) pour les access tokens ; les
refresh tokens ne sont pas exposés aux serveurs de ressources.

L'endpoint est monté uniquement lorsque `NVBES_IDENTITY_RESOURCE_SERVERS_JSON`
et le registre OAuth sont configurés. Le registre des ressources est un tableau
de un à huit objets contenant `client_id`, `audience` et `secret`. Les seules
audiences admises sont `nvbes-account-service` et `nvbes-billing-service`.
Le secret est constitué de 32 octets aléatoires encodés en base64url sans padding
(43 caractères). Les identifiants utilisent uniquement lettres ASCII, chiffres,
points, tirets et underscores. Aucun secret réel ne doit être versionné.

Chaque API utilise HTTP Basic avec son `client_id` et son secret. Ces credentials
sont distincts de ceux des clients OAuth publics. Les identifiants dupliqués et
les secrets réutilisés sont rejetés au démarrage ; deux credentials distincts
peuvent partager une audience pour permettre une rotation par configuration.
L'audience de recherche provient exclusivement du registre de la ressource
authentifiée, jamais du formulaire. En production, cet échange exige HTTPS.

Un jeton inconnu, révoqué, expiré ou destiné à une autre audience produit
uniquement `{"active":false}`. Une réponse active contient les identifiants,
scopes, audience, émetteur et dates du token ; `cnf` est fourni seulement pour
un jeton lié à une clé. Un résultat actif ne remplace pas la vérification de la
preuve DPoP par l'API ressource. Les erreurs de credentials produisent 401 ; une
panne du store ou un délai de consultation dépassant deux secondes produit 503.

Le corps est borné à 20 480 octets, le token à 16 384 caractères. Le quota source
protocolaire existant (120/minute, partagé par source réseau avec les autres
routes protocolaires) s'applique ; un dépassement produit 429. Toutes les réponses
interdisent le cache. La consultation relit grant, session, principal et registre
OAuth à chaque appel. Account et Billing utilisent désormais ce contrôle après
la validation locale de chaque JWT, sans cache positif ni repli en cas de panne.
Le SDK vérifie que la réponse active correspond aux identifiants,
dates, scopes, audience, issuer et confirmation du token vérifié localement.

La cible Nx `identity-service:test:resource-runtimes` vérifie cette intégration
avec les trois vrais binaires et trois bases isolées : Code/PKCE, lectures
métier, mauvaise audience, logout, arrêt et redémarrage d'Identity. La gestion
des cookies utilise Node sur loopback HTTP ; les politiques navigateur HTTPS,
DPoP et l'autorisation métier entre comptes restent des validations distinctes.

### Révocation d'une passkey

Chaque connexion primaire et step-up WebAuthn conserve le credential vérifié
dans `identity_session_webauthn_credentials`. Utiliser ensuite une autre clé
ne supprime pas cette association. Révoquer une clé révoque atomiquement toutes
les sessions associées et écrit l'audit ; une panne annule les deux mutations.
Cela peut terminer la session qui demande la révocation. La réponse confirme
l'opération, mais les appels suivants nécessitent alors une reconnexion.

OAuth, UserInfo, l'introspection et le refresh refusent les sessions révoquées.
Les API qui vérifient seulement la signature JWT doivent encore intégrer un
contrôle d'activité : la signature d'un jeton déjà émis reste valide jusqu'à
son expiration. Le logout intersites reste une capacité distincte à terminer.

La migration 0019 impose une reconnexion aux sessions antérieures au suivi
d'attribution 0018, ainsi qu'aux sessions WebAuthn sans attribution cohérente ou
liées à une clé déjà révoquée. Les anciennes preuves ne sont pas reconstituées
à partir d'indices. Un audit enregistre le nombre de sessions révoquées par compte.

## Autorisation directe et PAR

GET `/oauth/authorize` refuse les paramètres dupliqués après décodage des noms.
`max_age` accepte un entier décimal entre 0 et 86400. Les queries et les corps
PAR sont limités à 8192 octets et 32 paramètres. Les objets JAR via `request`
restent absents et leur présence provoque un refus explicite.

POST `/oauth/par` accepte le formulaire d'autorisation et retourne HTTP 201,
`request_uri` et `expires_in`. Il refuse un `request_uri` fourni dans son corps,
conformément à la [RFC 9126](https://www.rfc-editor.org/rfc/rfc9126.html#section-2.1).
Le navigateur présente ensuite uniquement `client_id` et `request_uri` à
`/oauth/authorize` ; tout paramètre supplémentaire est refusé dans ce profil.

La consommation PAR, la création d'une interaction unique et sa liaison au
navigateur sont atomiques. Un mauvais client, un cookie ambigu ou un échec SQL
pendant cette création/liaison ne détruit pas le PAR. Deux rédemptions concurrentes
n'en créent qu'une, et un
rejeu est refusé. La création directe et sa liaison ont la même atomicité.
En mode `prompt=none`, une panne PostgreSQL renvoie 503 sans la présenter comme
un besoin de reconnexion. Ces garanties ne prouvent pas encore le parcours
complet avec consentement, échange de code et consommateur de ressources.

## Renouvellement lié au client et à DPoP

POST `/oauth/token` avec `grant_type=refresh_token` exige `client_id` et
`refresh_token`. Le registre courant et le client du grant doivent correspondre.
Pour un grant lié à DPoP, chaque renouvellement exige une nouvelle preuve ES256
signée par la même clé, pour POST et l'URL canonique du token endpoint. Les preuves
manquantes, invalides, dupliquées ou rejouées renvoient `invalid_dpop_proof`.
La clé liée au grant ne peut pas être remplacée pendant un refresh.

La transaction verrouille d'abord le grant, puis le refresh. La consommation du
`jti`, la rotation du secret, l'émission et l'audit sont validés ensemble. Un
échec de signature annule ces changements. Les marqueurs DPoP sont conservés
jusqu'à la fin de la fenêtre de validité de la preuve, y compris la tolérance
d'horloge. Le rejeu d'un ancien secret avec le bon client et une nouvelle preuve
valide révoque sa famille et son grant. Une requête sans la bonne clé ne peut pas
déclencher cette révocation ; le rejeu de la seule preuve est refusé sans rotation.

Les jetons restent limités à la session active, aux scopes et à l'audience du
grant. Les réponses de jetons et erreurs de protocole portent `no-store` et
`no-cache`. Une indisponibilité PostgreSQL donne HTTP 503 et
`temporarily_unavailable`, sans émission de secours. Ce raccordement serveur
ne prouve pas encore le support DPoP des SDK et des serveurs de ressources.
La liaison du refresh à la clé suit la [RFC 9449, section 5](https://www.rfc-editor.org/rfc/rfc9449.html#section-5).

## Preuve CSRF des sessions hébergées

La création de session du login hébergé et sa liaison à l'interaction OAuth
forment une transaction unique avec l'audit et la rotation CSRF. Le mot de passe
est vérifié auparavant, sans transaction longue ; ses données de référence et
le statut du principal sont revérifiés sous verrou avant la création. Une
interaction refusée, rejouée ou une panne lors de sa liaison ne laisse pas de
session authentifiée orpheline.

Les réponses JSON de l'autorisation avec session active et du login fournissent
`session_csrf_token`. Le frontend Identity le renvoie dans `X-CSRF-Token`
pour POST `/oauth/logout` et POST `/oauth/session/step-up/totp`. Le champ
`csrf_token` reste réservé à l'interaction OAuth (login et consentement).
Ces valeurs restent en mémoire dans le site Identity ; elles ne sont pas placées
dans une URL ni transmises aux clients Account/Billing.

La preuve de session est un HMAC-SHA256 utilisant le secret opaque de session
comme clé et liant l'origine Identity et le cookie navigateur, avec séparation
de domaine. Le secret de session aléatoire de 256 bits reste dans un cookie
HttpOnly ; la preuve ne l'expose pas. Sa comparaison utilise la vérification
en temps constant de la bibliothèque HMAC. Elle change avec la session ou le
cookie navigateur et reste stable entre onglets d'une même session.

Le middleware vérifie l'origine, les cookies et la preuve avant de lire le JSON.
La validité cryptographique ne remplace pas l'état PostgreSQL : le step-up
verrouille la session et le principal et refuse un compte suspendu. Le logout
révoque uniquement la session présentée et écrit son audit dans la même
transaction ; le répéter ne duplique pas l'audit. Une panne de stockage retourne
503 sans prétendre avoir déconnecté la session. La déconnexion intersites
reste à terminer.

## Quotas des routes HTTP

Le démarrage avec `NVBES_IDENTITY_OAUTH_CLIENTS_JSON` exige aussi
`NVBES_IDENTITY_RATE_LIMIT_KEY` : une clé aléatoire distincte de 32 octets,
encodée en base64 standard, identique sur toutes les répliques. Une clé absente,
mal encodée ou nulle empêche le démarrage des routes publiques. Ne pas la renouveler
à chaque redémarrage : cela changerait l'affectation des compteurs.

Les routes authorize, login, approve, deny, logout, TOTP, token, PAR et UserInfo
comptent au maximum 120 requêtes par fenêtre de 60 secondes pour une même source.
Login et TOTP partagent aussi un plafond de 30 requêtes par source et par minute.
La source provient de la connexion TCP, sans son port. Les IPv4 mappées en IPv6
sont normalisées et les adresses IPv6 sont regroupées par /64.
`Forwarded`, `X-Forwarded-For` et les en-têtes CDN ne font pas autorité. Derrière
un proxy, son adresse partage donc le quota : l'exploitation doit valider cette
topologie avant ouverture, ou définir et tester un contrat de proxy de confiance.

Après validation de l'interaction, le login autorise cinq tentatives par email
normalisé sur 600 secondes, avant Argon2. TOTP utilise un quota distinct de cinq
tentatives sur 600 secondes par principal, partagé entre ses sessions.
Les succès comptent également et les échecs métier ne remboursent pas une tentative.

Les refus de quota retournent HTTP 429, `temporarily_unavailable`, `no-store`
et `Retry-After` (60 ou 600 secondes, délai conservateur). Une panne du quota ou
une attente supérieure à deux secondes retourne 503. Une absence d'information
de transport refuse aussi la requête. Les contrôles de source précèdent le parsing
des corps, la cryptographie et les mutations métier.

La migration 0014 ajoute la catégorie MFA sans modifier les compteurs existants.
Elle est compatible avec l'ancien binaire ; un retour applicatif peut conserver
la migration. PostgreSQL conserve au plus 4 × 4096 compteurs, sans email ni IP
en clair. Les collisions HMAC refusent de manière conservatrice. Aucun Redis
ni service payant supplémentaire n'est introduit ; la charge PostgreSQL et les
seuils restent à mesurer dans les gates de résilience et de budget.

## Preuves d'authentification

Le code capture les preuves de la session au moment de l'autorisation. Un MFA
réalisé ensuite ne renforce pas rétroactivement ce code. Une connexion par
passkey ne déclare pas `pwd`. Le MFA récent porte `step_up_time` et
`step_up_expires_at` ; les consommateurs devront contrôler leur fraîcheur au
moment de l'opération. La présence de `amr` seule n'est ni une preuve de fraîcheur
ni une attribution AAL3. Aucun `acr` ou sujet pairwise n'est annoncé.

La migration 0008 attribue `pwd` aux anciennes sessions, que seul l'authentificateur
par mot de passe pouvait créer. Elle retire les anciens step-ups dont la date
de preuve n'était pas enregistrée : une nouvelle vérification MFA sera nécessaire.
Les nouveaux enregistrements doivent fournir leur méthode et leur date. Les
anciens codes sans preuves enregistrées ne peuvent pas émettre de jetons.

## Rotation des clés sans service supplémentaire

La clé privée active est fournie par `NVBES_IDENTITY_TOKEN_PRIVATE_KEY_PEM`, son
identifiant par `NVBES_IDENTITY_TOKEN_KEY_ID`, et sa clé publique par
`NVBES_IDENTITY_TOKEN_PUBLIC_KEY_PEM`. Le démarrage vérifie que la paire correspond
et impose RSA 2048 à 4096 bits avec exposant 65537. Les structures contenant des
secrets n'implémentent pas `Debug`.

`NVBES_IDENTITY_TOKEN_VERIFICATION_KEYS` accepte un tableau JSON optionnel de
trois clés publiques supplémentaires au maximum : chaque objet contient `kid`,
`public_key_pem` et `accept_until` (secondes UTC depuis epoch). Aucun de ces objets
ne contient de clé privée. Les identifiants dupliqués sont refusés. À l'échéance,
la bibliothèque retire la clé du JWKS et de sa vérification, sans redémarrage.

La rotation opérateur suit ces étapes :

1. Ajouter la future clé publique au tableau, en conservant l'ancienne clé active.
2. Vérifier sa publication et attendre la durée maximale de cache des consommateurs.
3. Activer la nouvelle paire ; conserver l'ancienne clé publique dans le tableau
   jusqu'à l'expiration des jetons émis avec elle, en incluant la marge d'horloge
   et le délai de déploiement retenus pour les consommateurs.
4. Retirer l'ancienne clé après cette échéance et vérifier le JWKS.

Lors d'une compromission, retirer immédiatement la clé concernée et invalider
les autorisations touchées ; la rotation normale ne garantit pas une révocation
immédiate dans un consommateur qui utilise encore un ancien JWKS en cache.
Les politiques HTTP de cache, la révocation administrative et les parcours
consommateurs restent à raccorder avant ouverture publique.

Ces capacités utilisent la base Identity existante et des clés injectées dans
le runtime. Aucun Redis, KMS ni abonnement supplémentaire n'a été ajouté.
