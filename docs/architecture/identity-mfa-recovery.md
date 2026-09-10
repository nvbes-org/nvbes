# Récupération MFA par codes de secours

## État de livraison

Le noyau Rust et la migration 0022 sont implémentés et testés avec PostgreSQL
et de vraies signatures WebAuthn. Aucune route HTTP de récupération, interface
ou méthode SDK n'est encore montée. Les notifications sont des intentions
persistées dans l'outbox, pas des emails envoyés. Cette capacité n'est pas
déclarée disponible aux utilisateurs et ne clôture pas le lot B.

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
pas d'implémentation Debug. Le SDK devra les afficher une fois, sans persistance
navigateur ni télémétrie ; l'utilisateur devra les conserver hors ligne.

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

Les tests vérifient remplacement des codes, propriété du compte, fraîcheur,
suspension, consommation concurrente, rejet du jeton de récupération par OAuth,
expiration/remplacement des cérémonies, mauvaise origine signée, récupération
avec dix anciennes clés, utilisation cryptographique de la nouvelle clé et
rollback après panne de notification ou d'audit.

Avant activation HTTP : cookie de récupération HttpOnly/Secure distinct, preuve
CSRF dédiée liée au navigateur, Origin exact, corps bornés, quotas source/compte
durables et réponses non cachables. Le refus d'un code ne doit pas révéler son
existence ni réinitialiser les quotas en recréant une session. Compléter ensuite
SDK, interfaces, notifications réellement délivrées et preuve navigateur HTTPS.
Les gates d'exploitation et le plafond global de 30 EUR TTC/mois restent ouverts.
