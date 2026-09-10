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

Les tests PostgreSQL couvrent le refus du mot de passe, un vrai enrôlement et
une confirmation TOTP permettant de reprendre le même consentement, le refus
TOTP sous recent_webauthn, la limite d'expiration signée, le vieillissement du
snapshot et l'absence de rotation refresh lors d'un refus. Un durcissement entre
consentement et échange est refusé atomiquement ; le mode silencieux et
l'introspection ne contournent pas la nouvelle politique.

Les tests purs couvrent les échéances, le profil WebAuthn, les dates futures,
les valeurs inconnues et l'impossibilité de choisir un profil plus faible dans
une requête navigateur. Pas de migration ni nouvelle ressource payante. Aucun
client déployé n'est reconfiguré par cette tranche. Le parcours graphique de
sélection du facteur et l'application aux opérations administrateur restent
des exigences ouvertes du lot B.
