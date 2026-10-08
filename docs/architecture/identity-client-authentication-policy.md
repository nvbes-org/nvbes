# Politique d'authentification des clients OAuth

Le registre serveur des clients publics accepte `minimum_authentication`.
Le champ est géré avec les redirections, scopes et exigences DPoP ; aucun
paramètre navigateur ne peut le remplacer. Une valeur inconnue bloque le
chargement du registre. Son absence conserve explicitement le profil `primary`.

| Valeur            | Preuve nécessaire                                                                      |
| ----------------- | -------------------------------------------------------------------------------------- |
| `primary`         | Authentification primaire valide ; les contrôles sensibles des API restent applicables |
| `recent_mfa`      | Connexion WebAuthn ou step-up WebAuthn/TOTP datant de moins de cinq minutes            |
| `recent_webauthn` | Connexion ou step-up WebAuthn datant de moins de cinq minutes ; TOTP ne suffit pas     |

Les cérémonies WebAuthn actives exigent la vérification utilisateur. Cette
politique ne revendique aucune certification AAL/FAPI ni attestation matérielle.
Un mot de passe seul, un email ou un code de récupération n'accorde pas les
profils forts. Les dates futures/incohérentes restent refusées.

## Autorisation et durée de vie

Les consentements explicite et silencieux vérifient la politique actuelle.
Le hash des consentements inclut désormais ce profil : les consentements
antérieurs à ce changement devront être confirmés une nouvelle fois.
Un refus `access_denied` ne consomme pas l'interaction : le site Identity peut
effectuer le step-up avec le CSRF de session, puis réessayer le consentement
avec sa preuve d'interaction. `needs_login=false` signifie seulement qu'une
session primaire existe ; cela ne certifie pas que la politique forte est remplie.
Le choix et l'affichage du facteur dans l'interface restent à raccorder avant
d'activer ces profils sur un site destiné au public.

`POST /oauth/authorize/authentication` reçoit uniquement un objet `interaction`,
avec les cookies et le CSRF de cette interaction sur l'origine Identity. Sa
réponse non cachable fournit `minimum_authentication`, `needs_login`,
`needs_step_up` et `proof_expires_at` (null si aucune preuve forte actuelle).
Cette lecture ne consomme pas l'interaction, ne produit aucun code, ne rafraîchit
aucune preuve et ne pose aucun cookie. Le corps est limité à 4 KiB ; les champs
dupliqués/inconnus et tableaux sont refusés. La limite source OAuth s'applique.

Le SDK expose `getHostedAuthenticationStatus(transport, interaction)` et
`NvbesIdentityWeb.getAuthenticationStatus(interaction)`. Les interfaces relisent
cet état après login/step-up et avant affichage du consentement. Le statut ne
constitue pas un droit durable : le consentement relit les mêmes conditions.
Une session différente de celle liée à l'interaction demande un nouveau login.
Les erreurs réseau/stockage ne deviennent jamais un statut satisfait.

Les codes conservent le snapshot des preuves au consentement. L'émission, le
refresh, UserInfo et l'introspection relisent le registre actuel et vérifient
ce snapshot. Un step-up ultérieur dans la session ne rehausse pas un ancien
grant : obtenir une nouvelle autorisation. Un durcissement du registre peut
donc rendre inactifs des grants auparavant acceptés ; son déploiement doit être
coordonné entre toutes les instances Identity. Une relaxation opérateur explicite
réapplique le niveau moins strict, sans inventer de preuve dans les claims.

Pour un profil fort, l'expiration signée est au plus la date de la preuve plus
cinq minutes, bornée aussi par l'expiration du step-up, de la session et la durée
normale du jeton. Le refresh conserve cette échéance de preuve. Les serveurs qui
valident seulement le JWT ne voient un changement de registre qu'à l'expiration
du jeton ; les serveurs utilisant l'introspection reçoivent l'état courant.
La politique complète les autorisations par action dans Account/Billing ; elle
ne remplace ni les scopes ni les permissions d'opérateur.

## Validation et limites

Les opérations sensibles Account (demande/export du document, demande et
annulation de fermeture) revalident l'échéance forte signée sur l'horloge
PostgreSQL après leurs lectures/écritures bloquantes. La vérification précède
le commit ou la remise du document ; les réponses idempotentes sont aussi
concernées. La création éventuelle du profil et de ses préférences participe
à la transaction de demande : un refus annule ces écritures, l'audit et l'outbox.
Les scopes et l'introspection restent vérifiés à l'entrée HTTP. Ce contrôle
d'échéance n'ajoute pas d'atomicité distribuée avec une révocation Identity
survenant après l'introspection, ni de politique d'autorisation opérateur.

Les tests Account utilisent les routes HTTP réelles, des JWT signés et un
serveur d'introspection synthétique. Des verrous sur les tables d'audit,
d'outbox et de ressources prouvent le refus après expiration et l'absence
d'écritures persistées. Une preuve fraîche permet les mêmes opérations après
une attente réelle. Exécution via `account-service:test:database` avec une base
locale dédiée et les migrations actives ; aucun fournisseur externe requis.

Les terminaisons WebAuthn (connexion, step-up, enrollment) consomment le
challenge avec une condition d'expiration relue sur l'horloge PostgreSQL après
les attentes de verrou et la vérification cryptographique. Le step-up revalide
sa session et l'enrollment sa preuve récente après l'acquisition des verrous.
Un refus annule toute la transaction, y compris les compteurs de credential,
la session créée, ses liaisons OAuth et l'audit. Un contrôle effectué seulement
avant `SELECT ... FOR UPDATE` ne garantit pas que la preuve est encore valide
après une attente sur une ligne non modifiée.

Les tests retiennent explicitement un verrou de principal ou de challenge,
observent le waiter via `pg_blocking_pids`, attendent l'expiration selon
l'horloge de la base puis libèrent le verrou. Ils couvrent le challenge des
trois parcours, la session du step-up et de l'enrollment, la fraîcheur primaire
pour le premier facteur et la preuve forte pour un facteur supplémentaire.
Ce sont des preuves de transactions sous contention, pas un benchmark de charge.

Les mêmes refus s'appliquent à TOTP et à la récupération MFA. La confirmation
TOTP revalide l'autorisation après le verrou du facteur, puis active celui-ci
uniquement si son enrollment n'a pas expiré. La vérification du code de step-up
utilise l'heure PostgreSQL relue après ce verrou, sans heure fournie par
l'adaptateur HTTP. Les grants TOTP ne dépassent pas l'échéance de leur session.

La génération des codes de secours revalide la preuve forte après ses écritures
transactionnelles ; leur rédemption revalide la preuve primaire avant de
révoquer les sessions ordinaires. Le remplacement de facteurs relit la session
de récupération et consomme conditionnellement son challenge avant de terminer.
Une expiration annule aussi la consommation du code, les révocations, les
notifications et les nouveaux facteurs de la transaction concernée.

Les tests PostgreSQL couvrent le refus du mot de passe, un vrai enrôlement et
une confirmation TOTP permettant de reprendre le même consentement, le refus
TOTP sous recent_webauthn, la limite d'expiration signée, le vieillissement du
snapshot et l'absence de rotation refresh lors d'un refus. Un durcissement entre
consentement et échange est refusé atomiquement ; le mode silencieux et
l'introspection ne contournent pas la nouvelle politique.

Les tests purs couvrent les échéances, le profil WebAuthn, les dates futures,
les valeurs inconnues et l'impossibilité de choisir un profil plus faible dans
une requête navigateur.

Deux fixtures HTTPS neuves valident maintenant les profils dans Chromium
152.0.7977.83 avec les services Identity/Account et le SDK réels. Le scénario
runtime-browser-totp.mjs lit le statut anonyme, effectue le login, vérifie le
refus du consentement après mot de passe, puis le statut après TOTP. Pour
recent_webauthn, le consentement reste refusé après TOTP jusqu'à une assertion
WebAuthn avec vérification utilisateur. Les deux profils terminent par un
échange OIDC/DPoP et un accès Account autorisé. Les preuves ne sortent pas dans
le rapport du test.

Lancer une fixture neuve avec la cible identity-service:test:https-browser-fixture,
puis appeler verifyBrowserTotp(browser, clientOrigin, false, 'recent_mfa') ou
verifyBrowserTotp(browser, clientOrigin, false, 'recent_webauthn'). Chaque profil
utilise son client enregistré et exige une fixture neuve, car le scénario
enrôle un premier facteur. Arrêter la fixture ensuite. Seuls le calcul TOTP et
l'authentificateur WebAuthn sont simulés ; aucune clé physique ni application
TOTP mobile n'est attestée. Le test interactif ne tourne pas en CI.

Pas de migration ni nouvelle ressource payante. Aucun client déployé n'est
reconfiguré. Les écrans de sélection du facteur et l'application aux opérations
administrateur restent des exigences ouvertes du lot B.
