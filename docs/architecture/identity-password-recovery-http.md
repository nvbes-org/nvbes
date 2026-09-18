# Récupération du mot de passe : HTTP, SDK et web

Le parcours utilise la [file Email chiffrée](identity-password-recovery-delivery.md)
et le reset transactionnel existants. Il conserve l'habillage Identity archivé.
Il ne crée pas de session et ne remplace aucune preuve MFA.

## Activation et routes

Le serveur monte ces routes uniquement avec
`NVBES_IDENTITY_PASSWORD_RECOVERY_ENABLED=1`, une origine navigateur HTTPS
valide et les prérequis du runtime OAuth existant. La migration 0025 ajoute
des quotas séparés. Le serveur de fichiers doit servir Identity Web sur
`/password-recovery`. Sans activation HTTP, cette page indique que la
récupération est indisponible.

| Méthode et chemin                       | Contrat                                                                                                       |
| --------------------------------------- | ------------------------------------------------------------------------------------------------------------- |
| GET `/oauth/password-recovery/context`  | Cookie navigateur hôte et preuve CSRF liée à ce cookie ; aucune mutation de compte                            |
| POST `/oauth/password-recovery/request` | `{email}` ; réponse 202 `{accepted:true}` pour adresse éligible, inconnue ou compte inactif                   |
| POST `/oauth/password-recovery/reset`   | `{token,password}` ; après commit, `{reset:true,must_reauthenticate:true}` et effacement du cookie de session |

Une réponse 202 reconnaît la demande, pas la livraison d'un email. La commande
opérateur de dispatch reste nécessaire. Aucun envoi fournisseur, activation
en production ni cadence automatique n'est effectué par ce changement.

## Frontière navigateur et limites

Les POST exigent une origine exacte, JSON, des Fetch Metadata compatibles et
un header CSRF HMAC lié au cookie navigateur. Les cookies ambigus et champs
JSON dupliqués/inconnus sont refusés. La taille du corps est limitée à 4 Kio.
Toutes les réponses sont privées (`no-store`, `no-referrer`, protection iframe).

L'adresse source vient de la connexion, jamais d'un header forwarded non
authentifié. Les IPv4 mappées sont normalisées, les IPv6 groupées par /64.
Une topologie reverse proxy doit conserver sa frontière de confiance démontrée
avant activation ; les limites peuvent sinon regrouper ses utilisateurs.

- 30 mutations par source et par fenêtre de 15 minutes.
- 3 demandes par email normalisé et par fenêtre de 15 minutes, compte existant
  ou non ; aucune consommation du quota de connexion.
- 5 essais par jeton de reset et par fenêtre de 15 minutes.
- 16 opérations simultanées par processus, délai de 2 secondes sur les quotas
  et 5 secondes sur le traitement du handler.

Les sujets de quota sont HMACés dans les buckets bornés existants. Les
collisions refusent conservativement. Un plancher de réponse de 250 ms réduit
la différence ordinaire entre adresse connue et inconnue ; ce n'est pas une
preuve de temps constant sous contention. Une panne du store donne 503,
jamais un faux accusé 202. Les volumes et délais restent à mesurer en charge.

## Secrets et comportement utilisateur

Le lien Email utilise `#token=...`, sans query. Les destinations configurées
avec query, fragment ou credentials sont refusées. Le navigateur retire le
fragment avant le bootstrap HTTP et conserve le token dans le contrôleur,
jamais dans le stockage web ni son état affichable. Les liens ambigus sont
refusés. Une ouverture dans le même document relance le bootstrap.

Les champs mot de passe sont vidés avant l'appel SDK. La soumission, le départ
de page et l'échec effacent la capacité détenue en mémoire. Aucun reset n'est
réessayé automatiquement. Un accusé perdu peut correspondre à un commit réussi :
l'interface invite à vérifier l'accès depuis l'application ou demander un
nouveau lien, sans annoncer un succès non confirmé.

Le succès révoque les sessions et tous les anciens liens dans PostgreSQL, puis
demande une nouvelle connexion. Les facteurs MFA ne sont pas supprimés et un
reset ne confère pas d'AMR fort. Il n'existe aucun retour arbitraire fourni par
la query ou le lien.

## Validation et limites restantes

Tests HTTP avec migrations réelles : réponses d'admission identiques, quotas,
origine et preuve incorrectes, champs dupliqués, panne du store, reset, rejeu
et invalidation des sessions. Tests SDK : forme des accusés, refus cross-origin,
absence de retry et transport des secrets dans le corps POST uniquement.
Tests React : effacement des champs, double soumission et résultat tardif.

La fixture navigateur lit uniquement sa file synthétique chiffrée pour simuler
l'ouverture du lien reçu. Elle ne prouve pas la livraison fournisseur. Les
notifications après changement sont enregistrées atomiquement dans la file de
sécurité décrite dans le contrat de livraison. Les preuves d'exploitation et
les autres exigences des lots A–D restent ouvertes.
