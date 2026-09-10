# Jetons du service Identity actif

Ce document décrit la bibliothèque et les routes OAuth montées sous configuration
dans le service actif. Les parcours complets et les consommateurs Account/Billing
restent à terminer dans les [lots A à D](identity-protocol-implementation.work.md).

## Émission et révocation

L'échange d'un code produit une capacité d'émission liée au grant PostgreSQL.
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

La durée de vie est plafonnée à 900 secondes et à l'expiration de la session.
La vérification cryptographique seule ne lit pas l'état PostgreSQL. L'introspection
relit le grant, la session, le principal et le registre courant : le rejeu valide
d'un code invalide donc aussi les jetons déjà émis. En cas de panne du stockage,
l'introspection renvoie une erreur, jamais un résultat actif par défaut. Les
consommateurs exigeant une autorisation actuelle devront refuser l'opération si
ce contrôle est indisponible. Leur intégration fait encore partie des lots.

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
503 sans prétendre avoir déconnecté la session. La déconnexion intersites et
les limites de tentatives sur les parcours HTTP restent à terminer.

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
