# Enrôlement TOTP hébergé

## Contrat actif

Les routes sont montées avec le protocole navigateur Identity, sous sa configuration
existante. Elles exigent cookie de session, cookie navigateur, Origin exact et
preuve CSRF de session. Aucun Bearer ni CORS interorigine n'est utilisé.

| Route POST                               | Corps JSON                                            | Réponse 200                                                    |
| ---------------------------------------- | ----------------------------------------------------- | -------------------------------------------------------------- |
| `/oauth/session/totp/enrollment/start`   | `{}`                                                  | `factor_id`, `secret_base32`, `provisioning_uri`, `expires_at` |
| `/oauth/session/totp/enrollment/confirm` | `factor_id` UUID, `code` chaîne de six chiffres ASCII | `enrolled: true`, `step_up: true`, `expires_at`                |
| `/oauth/session/totp/factors/list`       | `{}`                                                  | Tableau de zéro ou un facteur actif : `id`, `created_at`       |
| `/oauth/session/totp/factors/revoke`     | `factor_id` UUID                                      | `revoked: true`, `sessions_revoked: true`                      |

Les corps sont bornés à 4096 octets. Seuls les objets sont acceptés ; tableaux,
champs inconnus et champs dupliqués sont refusés. Les réponses portent `no-store`
et `no-referrer`. La réponse de démarrage contient le secret de provisioning :
elle ne doit être ni journalisée, ni persistée par l'interface, ni envoyée à un
générateur de QR externe. L'URI utilise le profil TOTP actif : SHA1, six chiffres,
période de trente secondes, issuer `nvbes` et UUID du principal comme compte.

Le quota source est partagé avec les autres opérations d'authentification
(30/minute, plus quota protocole de 120/minute). Démarrage, confirmation et révocation
consomment le quota TOTP existant de cinq requêtes par principal sur dix minutes,
partagé entre sessions et avec le step-up TOTP. Les échecs ne remboursent pas
le compteur. La vérification Origin/CSRF précède le traitement du corps.
La liste consomme seulement les quotas source, sans épuiser les tentatives TOTP.

Les refus métier renvoient 400 `invalid_request`, les quotas 429 avec Retry-After,
les pannes de persistance/crypto 503. Les erreurs n'exposent aucun secret.

## Persistance et authentification

La migration 0021 ajoute la session d'enrôlement et son expiration aux facteurs
existants. Elle ne modifie aucun facteur actif. Un ancien facteur pending sans
liaison navigateur ne peut pas être confirmé par ces nouvelles routes. Un retour
à l'ancien binaire peut conserver les colonnes supplémentaires ; aucune suppression
de colonne ou réécriture des compteurs n'est nécessaire.

Il existe au plus un facteur TOTP par principal. Un démarrage remplace un pending
ou un facteur révoqué avec un nouvel identifiant et un nouveau secret chiffré ;
il ne remplace jamais un facteur actif. L'ancien identifiant devient inutilisable.
La confirmation exige la même session, le même principal et une expiration future
(cinq minutes). Un pending expiré reste inactif et sera remplacé au prochain
démarrage ; il n'accumule pas de lignes supplémentaires.

La politique partagée avec WebAuthn verrouille le principal avant la session :
authentification primaire récente pour le premier facteur, preuve forte récente
si un facteur fort existe déjà. Elle est revérifiée à la confirmation. Une session
révoquée, un principal suspendu ou une preuve trop ancienne ne peuvent confirmer.

Le secret utilise AES-256-GCM via MfaCrypto, avec nonce, version de clé et données
associées liées à l'identifiant du facteur. La confirmation active le facteur,
consomme le compteur TOTP, accorde dix minutes de step-up et écrit les audits
dans une seule transaction. Une panne d'audit annule ces changements. Le code de
confirmation ne peut pas être réutilisé pour un step-up ; aucune preuve `pwd`
supplémentaire n'est inventée.

## Consultation et révocation

La liste exige une session active mais pas de step-up récent. Elle ne retourne
ni secret, ni compteur, ni facteur pending/révoqué. La révocation exige une preuve
forte de moins de cinq minutes et une passkey active en remplacement ; supprimer
le dernier facteur fort renvoie 409 `last_strong_factor`. Une session au seul mot
de passe, un autre propriétaire ou une preuve périmée sont refusés.

La politique et l'ordre de verrouillage sont partagés avec la gestion WebAuthn.
Deux suppressions concurrentes TOTP/passkey ne peuvent pas éliminer tous les
facteurs. La révocation efface le ciphertext TOTP, marque le facteur révoqué,
révoque toutes les sessions du principal et écrit l'audit dans une transaction.
Ce choix couvre aussi les sessions ayant utilisé TOTP avant un autre step-up.
Les API et refresh déjà liés à ces sessions deviennent inutilisables via les
contrôles de session existants ; les sites doivent engager une reconnexion.
La propagation proactive de l'interface reste un chantier distinct.

Un nouvel appel sur le même facteur déjà révoqué est sans effet avec une nouvelle
session autorisée ; l'ancienne session du demandeur ne peut pas effectuer de retry.
Les secrets révoqués sont exclus de la rotation des clés MFA. Les autres facteurs
pending/actifs continuent à être rechiffrés. Réenrôler exige une nouvelle preuve
forte et remplace la ligne révoquée par un identifiant et un secret neufs.

## Validation et limites

Tests actifs HTTP/PostgreSQL : démarrage/confirmation, refus du rejeu, CSRF et
Origin, quotas et corps bornés, JSON ambigu, secret chiffré, remplacement d'un
pending, mauvaise session, expiration, révocation/suspension, concurrence et
rollback après échec d'audit. Les tests WebAuthn existants et le parcours Chromium
passent avec la politique d'enrôlement commune.

Le SDK couvre enrôlement, confirmation, step-up, liste et révocation. Les preuves
Chromium HTTPS utilisent les vrais services et vérifient le refus du dernier
facteur et de l'accès Account après révocation. L'authentificateur est synthétique.

L'interface TOTP, la récupération MFA,
les notifications et la politique opérateur restent à terminer. Ces routes ne
constituent pas un parcours TOTP produit complet. Aucune infrastructure nouvelle
ni service payant n'est ajouté ; les gates d'exploitation et du budget global
restent ouverts.
