# Récupération MFA par codes de secours

## État de livraison

Le noyau Rust, la migration 0022 et les routes HTTP sont implémentés et testés
avec PostgreSQL et de vraies signatures WebAuthn. Les routes utilisent la même
configuration d'activation que les opérations OAuth hébergées ; aucune ouverture
publique ou infrastructure n'est effectuée par ce changement. Les méthodes SDK
sont raccordées et testées, y compris un parcours Chromium HTTPS avec les
services réels et des authentificateurs virtuels. L'interface reste à livrer.
Les notifications sont des intentions
persistées dans l'outbox, pas des emails envoyés. Cette capacité n'est pas
déclarée complète pour les utilisateurs et ne clôture pas le lot B.

## Contrat HTTP

Toutes les routes utilisent POST JSON, Origin exact et cookies same-origin.
Les réponses sont non cachables, sans referrer ; aucun Bearer ou CORS n'est
accepté pour effectuer ces opérations.

| Route                                    | Preuve requise                        | Corps objet                          | Réponse 200                                                     |
| ---------------------------------------- | ------------------------------------- | ------------------------------------ | --------------------------------------------------------------- |
| `/oauth/session/recovery/codes/generate` | Session normale et CSRF de session    | `{}`                                 | `codes` : dix chaînes                                           |
| `/oauth/session/recovery/redeem`         | Session normale et CSRF de session    | `code`                               | `recovery: true`, `csrf_token`, `expires_at`                    |
| `/oauth/recovery/registration/options`   | Cookie de récupération et CSRF dédiée | `{}`                                 | `ceremony_id`, `options` WebAuthn                               |
| `/oauth/recovery/registration/finish`    | Cookie de récupération et CSRF dédiée | `ceremony_id`, `credential`, `label` | `recovered: true`, `credential_id`, `must_reauthenticate: true` |

`POST /oauth/recovery/resume` reçoit `{}` avec les cookies navigateur/récupération,
Origin exact et un nonce de 43 caractères dans X-CSRF-Token. Cette lecture
restitue `recovery: true`, `csrf_token`, `expires_at`. Le nonce est un header
personnalisé, pas une preuve d'autorisation ; Origin, JSON et Fetch Metadata
protègent l'accès same-origin au cookie HttpOnly. Le principal et la récupération
doivent être actifs. Aucun Set-Cookie ni renouvellement d'expiration n'a lieu.
Cette amorce ne doit jamais autoriser une mutation.

`POST /oauth/recovery/cancel` reçoit `{}` avec la véritable preuve CSRF de
récupération et retourne `cancelled: true`, `must_reauthenticate: true`.
Il supprime récupération/cérémonie par cascade et écrit audit et intention
`identity.mfa_recovery_cancelled` atomiquement. Seul le cookie de récupération
est effacé après commit. Le code reste consommé, les sessions révoquées et les
facteurs inchangés. Reprise/abandon d'une récupération expirée, annulée ou
remplacée renvoie 400 ; une panne renvoie 503 sans effacer le cookie.

La consommation réussie pose `__Host-nvbes-recovery` (HttpOnly, Secure,
SameSite=Lax, Path=/, Max-Age=300, sans Domain) et efface le cookie de session
ordinaire. Le jeton de récupération n'apparaît pas dans le JSON. La preuve CSRF
est un HMAC du navigateur et de l'origine avec un domaine distinct du CSRF de
session. Le succès final efface les cookies de récupération et de session.
Une panne avant commit ne pose ni n'efface de cookie.

Les objets refusent champs inconnus, champs dupliqués et tableaux positionnels.
Les corps sont limités à 4096 octets, sauf la réponse d'attestation finale
(65 536 octets). Les refus métier renvoient 400, les mauvaises preuves navigateur
403, les quotas 429 avec Retry-After=600 et les pannes 503.

Le quota source de connexion (30/minute) et le quota protocole (120/minute)
précèdent l'analyse navigateur. Génération et consommation utilisent le quota
MFA partagé de cinq requêtes par principal sur dix minutes ; options, finish,
reprise et abandon utilisent le quota WebAuthn de vingt requêtes. Recréer une session normale ou de
récupération ne réinitialise pas ces compteurs. Les corps invalides sont comptés.
Ces plafonds sont partagés avec les autres opérations MFA/WebAuthn existantes.

## Contrat et séparation des preuves

La récupération MFA reste distincte du reset de mot de passe par email.
Un code ne devient jamais `amr=webauthn`, `amr=totp` ni un step-up. La
table `identity_mfa_recovery_sessions` est distincte de `identity_sessions` :
son jeton ne peut pas servir au consentement OAuth ni aux routes de gestion
ordinaires. Il autorise uniquement une cérémonie de remplacement WebAuthn.

Le parcours cible du runtime actuel est :

1. Depuis une session avec preuve forte de moins de cinq minutes, générer dix
   codes. Au moins un facteur fort actif doit exister. Une nouvelle génération
   remplace les codes précédents et annule les récupérations en attente.
2. Après perte du facteur, effectuer une connexion primaire récente et présenter
   un code. La session doit être active et le principal non suspendu. Le code
   est consommé atomiquement et toutes les sessions ordinaires sont révoquées.
3. Utiliser le jeton de récupération de cinq minutes pour enregistrer une nouvelle
   passkey. Aucun OAuth ou accès métier n'est accordé par ce jeton.
4. Une fois l'attestation vérifiée, remplacer les facteurs : révoquer toutes les
   anciennes clés, effacer les secrets TOTP, invalider les codes restants et les
   sessions, puis consommer l'autorisation de récupération dans une transaction.
5. Effectuer une nouvelle connexion avec le facteur enregistré et générer un
   nouveau jeu de codes après preuve forte récente.

Cette version exige donc de pouvoir encore réaliser une connexion primaire
(notamment avec le mot de passe du runtime actuel). Elle ne résout pas à elle
seule la perte simultanée du mot de passe et de tous les facteurs ; le parcours
de récupération du mot de passe et les comptes exclusivement passwordless
nécessitent leur propre politique avant exposition publique.

## Secrets, concurrence et persistance

Chaque code contient 32 octets aléatoires encodés en base64url sans padding,
préfixés `nvr1_` : 48 caractères, 256 bits d'entropie. Seul SHA-256 du code avec
séparation de domaine et UUID du principal est stocké. Il n'y a ni code en clair
en base, ni code dans les événements. Les réponses contenant les secrets n'ont
pas d'implémentation Debug. Le SDK retourne les codes sans les persister ni les
journaliser ; l'interface devra les afficher une fois pour sauvegarde hors ligne.

Le verrou du principal précède ceux des sessions, codes et cérémonies. Deux
consommations concurrentes du même code ne réussissent pas. Une récupération
en remplace une précédente pour le même principal ; les anciens challenges
sont supprimés par cascade. Le stockage reste borné à dix codes, une session
de récupération et une cérémonie en attente par principal.

Le remplacement conserve les anciens facteurs tant que la nouvelle attestation
n'est pas vérifiée. Même un compte ayant déjà dix passkeys peut récupérer :
les anciennes sont révoquées dans la transaction qui insère la nouvelle. Les
identifiants existants sont exclus de la cérémonie et leur unicité reste imposée.
La clé nouvelle doit ensuite produire une assertion lors de la reconnexion.

Une panne d'audit, d'outbox ou de persistance annule toute la transaction concernée.
Un échec final ne détruit donc pas les anciens facteurs. Les sessions créées par
une connexion concurrente pendant la récupération sont aussi révoquées à sa fin.
Les événements `identity.mfa_recovery_codes_generated`,
`identity.mfa_recovery_started` et `identity.mfa_recovered` ne contiennent que
l'identifiant du principal et doivent être traités par le futur dispatcher.

## Validation et raccordements requis

Le contrat Email AccountSecurityV1 dispose désormais d'événements distincts
MfaRecoveryCodesGenerated, MfaRecoveryStarted, MfaRecovered et MfaRecoveryCancelled.
Les messages texte/HTML décrivent les effets réels, sans code ni jeton ; notamment
MfaRecovered ne prétend pas qu'un mot de passe a été changé. Le constructeur
Identity fixe idempotence et échéance à partir de l'événement (24 heures), mais
le dispatcher d'outbox et la livraison restent à raccorder. L'adresse vérifiée
doit être figée avant le premier envoi pour conserver la même commande aux retries.
Le consommateur Email compatible doit précéder l'activation de ce producteur.

Les tests vérifient remplacement des codes, propriété du compte, fraîcheur,
suspension, consommation concurrente, rejet du jeton de récupération par OAuth,
expiration/remplacement des cérémonies, mauvaise origine signée, récupération
avec dix anciennes clés, utilisation cryptographique de la nouvelle clé et
rollback après panne de notification ou d'audit.

Les tests HTTP vérifient aussi la séparation des cookies/CSRF, le cycle complet,
les corps ambigus, l'épuisement des quotas entre sessions, les erreurs de stockage
et l'absence de modification des cookies après rollback. Les tests SDK couvrent
le contrat actif, les preuves distinctes, les réponses malformées, l'expiration,
l'annulation native et les erreurs sans retry ni fuite de secrets. Le scénario
runtime-browser-recovery.mjs vérifie en HTTPS la révocation de l'accès Account,
les cookies isolés, l'annulation, le remplacement, la reconnexion par la nouvelle
clé et un nouvel accès Account OIDC/DPoP. Il ne prouve pas la compatibilité des
clés physiques ni celle de tous les navigateurs. Compléter ensuite interfaces
et notifications réellement délivrées.
La reprise après un vrai rechargement et l'abandon explicite sont aussi vérifiés
dans Chromium ; l'autorisation expire toujours au bout de cinq minutes.
Les gates d'exploitation et le plafond global de 30 EUR TTC/mois restent ouverts.
