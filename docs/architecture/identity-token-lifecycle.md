# Jetons du service Identity actif

Ce document décrit la bibliothèque implémentée dans le service actif. Les routes
publiques et les consommateurs Account/Billing doivent encore être raccordés
dans les [lots A à D](identity-protocol-implementation.work.md).

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
