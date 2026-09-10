# Déconnexion initiée par un client OIDC

## Parcours livré

Account et Identity exécutent le profil web de
[OpenID Connect RP-Initiated Logout 1.0](https://openid.net/specs/openid-connect-rpinitiated-1_0.html).
Account efface ses accès locaux puis soumet un formulaire POST à
`/oauth/end-session`, avec son ID Token, son client enregistré, son callback
de logout et un state aléatoire. Le serveur accepte également GET pour les
clients OIDC utilisant ce transport. Aucun de ces appels ne révoque de session.

Identity vérifie le hint et la destination, puis répond HTTP 303 vers
`/logout#request=...`. Le contexte signé contient uniquement la demande validée,
sans ID Token ni secret de cookie. Il expire après cinq minutes, possède un type
et une audience distincts des jetons d'authentification, et utilise le trousseau
de signature Identity existant. Aucun service ou stockage supplémentaire.

Le site Identity retire immédiatement le fragment de l'historique et garde le
contexte en mémoire. Il charge la preuve CSRF de session sur la même origine,
puis prépare la confirmation. Seul le clic explicite révoque la session. Annuler
ne révoque rien ; recharger abandonne la demande RP en mémoire et permet encore
le logout local existant, sans retour externe.

Après succès, Account consomme et vérifie son state avant d'afficher la déconnexion.
Seuls state, échéance et adresse de callback ont été conservés en sessionStorage.
Un callback invalide, expiré ou rejoué est refusé et son URL est nettoyée avant
le rendu. Le formulaire temporaire contenant l'ID Token est supprimé du DOM.

## Contrat HTTP

| Endpoint                            | Entrée                       | Protection et résultat                                                                |
| ----------------------------------- | ---------------------------- | ------------------------------------------------------------------------------------- |
| GET/POST `/oauth/end-session`       | Query ou formulaire OIDC     | Quota source, doublons refusés, politique client et signature ; 303 interne seulement |
| GET `/oauth/session/logout-context` | Cookies et header de lecture | Même origine, preuve CSRF liée aux cookies ; aucune révocation                        |
| POST `/oauth/logout/prepare`        | JSON request et CSRF         | Session et contexte vérifiés ; prepared: true                                         |
| POST `/oauth/logout/confirm`        | JSON request et CSRF         | Révocation et audit atomiques ; logged_out: true et redirect_uri ou null              |

Les réponses portent no-store, no-referrer et les protections navigateur Identity.
Les deux POST JSON exigent Origin, cookies et CSRF avant lecture du corps. Les
corps sont bornés à 32 Kio, les contextes signés à 16 Kio et state à 1 Kio.
Les échecs ne transmettent aucune destination externe et ne suppriment pas le
cookie. Les clients GET doivent exclure les query strings de leurs journaux
d'accès ; le client Account livré utilise POST et ne met pas l'ID Token dans l'URL.

## Invariants

- Un ID Token expiré peut être un indice signé, jamais une autorisation API.
  Signature RS256, issuer, type JWT, clé encore admise et claims sont vérifiés.
- Le client doit toujours être enregistré. Le retour exige un hint et une
  égalité exacte avec une entrée post_logout_redirect_uris. Un client_id public
  seul ne justifie pas de retour dans ce profil.
- Le principal et le sid du hint doivent correspondre à la session du cookie.
  Une session expirée depuis moins d'une heure peut encore être fermée ; aucune
  session d'un autre navigateur n'est révoquée à partir d'un hint seul.
- Préparation et confirmation vérifient le contexte signé et la configuration
  client courante. La confirmation verrouille la session dans la transaction.
  Deux confirmations concurrentes peuvent acquitter la même révocation, avec
  un seul événement d'audit. Une panne d'audit annule toute la transaction.
- La découverte annonce end_session_endpoint seulement lorsque le runtime
  possède le registre client et la configuration navigateur nécessaires.

## Configuration des sites

Enregistrer l'adresse exacte `<origine Account>/oauth/logout/callback` dans
post_logout_redirect_uris du client Account. Servir le document Account sur
cette route. Autoriser uniquement l'origine Identity configurée dans la directive
CSP form-action d'Account, en plus de self. La fixture HTTPS applique ces règles.
Le SDK utilise une transaction de cinq minutes et demande un callback sans query.
Les origines de production restent HTTPS ; HTTP loopback est réservé au profil
de développement déjà validé par le registre.

Les tâches Nx test/typecheck/lint des deux sites dépendent du build du SDK : elles
ne peuvent plus démarrer pendant que son répertoire de sortie est reconstruit.

## Preuves et limites

Les tests unitaires couvrent signatures, clés retirées, expiration/type du contexte,
claims incohérents, doublons, state et destinations. Les tests PostgreSQL réels
couvrent GET/POST, absence de révocation à la préparation, concurrence, autre
session, changement de registre et pannes de stockage/audit. Le parcours HTTPS
utilise les builds réels Identity/Account, Chromium et les trois services.

La fixture vérifie annulation, confirmation, retour Account, state consommé,
ancien accès Billing refusé, refresh refusé et reconnexion requise. Sa variante
avec focus réel retire aussi le profil d'une seconde fenêtre Account à son retour.

Les notifications back-channel proactives restent ouvertes : une fenêtre
continuellement au premier plan n'est pas notifiée en temps réel. Aucun transport
de notification RP n'est enregistré dans le profil courant. Ce lot ne démontre
ni certification OIDC/FAPI, ni préparation complète à l'ouverture publique, ni
achèvement des autres exigences A–D.
