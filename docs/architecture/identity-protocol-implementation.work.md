# Exécution des lots Identity A à D

## Objectif et périmètre conservé

Implémenter intégralement les lots A à D de
[l'inventaire](identity-archive-mechanisms-review.md), dans les services actifs,
avec migrations réelles et tests adversariaux. Ce suivi ne remplace pas les
exigences du document source et n'autorise aucune ouverture partielle en public.

Branche : `codex/frontend-platform-preparation`, PR 210. Worktree isolé :
`/Users/shayn/Development/nvbes-identity-protocol` (le checkout principal est
utilisé en parallèle). La branche de PR Identity 175, commit `890e0b7e`, a été
intégrée localement comme dépendance par merge signé `ce7adc9b`. Aucune PR n'a
été fusionnée dans main et aucune infrastructure n'a été déployée.

## File durable et dispatcher des notifications MFA — 2026-09-10

La migration 0023 et les modules queue/dispatch figent les destinataires vérifiés
dans la transaction MFA. La clé d'idempotence inclut désormais le hash du
destinataire pour notifier plusieurs emails sans conflit. La commande opérateur
dispatch-security-notifications soumet au plus seize commandes à Email, avec
baux persistés de soixante secondes, timeout vingt secondes, huit tentatives et
backoff borné. Les adresses sont effacées à l'état terminal ; les métadonnées
sont purgées après trente jours par lots bornés. Le reçu persiste uniquement
une acceptation Email, pas une livraison. Aucun scheduler nouveau.

Les états sans adresse vérifiée ou sans snapshot historique sont explicites.
Les tests PostgreSQL couvrent concurrence, reprise avec commande stable après
changement d'adresse, disparition du processus, fencing des réponses tardives,
reçu incorrect, expiration/épuisement, enqueue MFA réel et rollback, rétention.
Le [contrat d'exploitation](identity-security-notifications.md) précise les
limites et le rapprochement avec Email. La preuve réseau avec le service Email,
la livraison fournisseur et les gates d'exploitation restent ouverts.

La base de test actuelle est nvbes_identity_test_notifications sur le conteneur
local nvbes-identity-protocol-tests (port 15433). Elle remplace le nom précédent
pour repartir d'un schéma neuf après finalisation de la migration non déployée.
Le transport est substitué dans les tests du dispatcher ; aucun email réel
n'est envoyé. Les interfaces, politiques opérateur et autres exigences A à D
restent ouvertes.

Validation finale : cargo check --workspace sans avertissement ; 193 tests
bibliothèque Identity, 24 tests runtime et trois gardes PostgreSQL passent.
Un test navigateur interactif reste ignoré dans Cargo. Aucun frontend ou SDK
modifié, donc pas de nouvelle exécution navigateur pour cette tranche.

## Contrat des notifications de récupération MFA — 2026-09-10

Le contrat Email possède maintenant quatre événements MFA distincts (valeurs
Protobuf 5 à 8), avec encodage/décodage et rendus texte/HTML. L'ancien événement
AccountRecovered annonce un reset de mot de passe : il n'est donc pas réutilisé
pour la récupération MFA. Les nouveaux messages distinguent génération des
codes, début, remplacement effectif des facteurs et abandon. Ils n'incluent ni
code, jeton, identifiant de facteur ni lien d'authentification.

Le constructeur Identity accepte exclusivement ces quatre événements, une ligne
d'outbox de confiance et un destinataire dont la vérification devra être garantie
par le dispatcher. Il produit une commande AccountSecurity, une clé d'idempotence
liée à l'événement et une échéance fixe de 24 heures après son occurrence.
Un retry ne prolonge pas l'échéance ; événements futurs/expirés et destinataires
invalides sont refusés. Le passage Protobuf aller-retour et les textes exacts
sont testés côté Identity.

Validation : cargo check --workspace, cible Nx email-worker:test (bibliothèque,
worker, adaptateur Scaleway), 188 tests bibliothèque Identity, 24 runtime et trois
gardes PostgreSQL passent ; un test interactif reste ignoré. Aucun frontend ou
parcours navigateur modifié, aucun envoi réseau réel effectué par cette tranche.

Le dispatcher durable reste à implémenter : figer le destinataire vérifié avant
le premier envoi, réclamer les événements sans concurrence, borner retries et
backoff, persister le reçu Email, distinguer acceptation et livraison finale,
traiter expiration/absence d'adresse et tester redémarrage/pannes. Une ancienne
version d'Email refuse les nouvelles valeurs : déployer le consommateur compatible
avant d'activer le producteur. Aucun déploiement n'est réalisé ici. Notifications
effectivement délivrées, interfaces et autres exigences A à D restent ouvertes.

## Reprise et abandon de récupération MFA — 2026-09-10

Les routes resume/cancel et leurs méthodes SDK complètent le cycle de vie.
La reprise est une lecture same-origin du cookie HttpOnly avec JSON et header
personnalisé ; elle restitue la preuve CSRF dédiée sans modifier l'expiration.
L'abandon exige cette preuve et supprime récupération/cérémonie atomiquement
avec audit et intention de notification, sans restaurer code ou sessions.
Les contrôles existants de source, compte, taille et objets stricts s'appliquent.

Les tests PostgreSQL vérifient expiration constante, principal suspendu,
récupération expirée, refus de la preuve d'amorçage sur l'abandon, rejet de
l'attestation après abandon, conservation du facteur et du code consommé,
quotas et rollback après panne de l'outbox. Chromium 152.0.7977.83 exécute le
parcours HTTPS complet avec rechargement réel, récupération de la preuve,
remplacement puis nouvelle récupération annulée ; Account demeure révoqué.

Validation : cargo check --workspace, 186 tests bibliothèque Identity, 24 runtime,
trois gardes PostgreSQL, 120 tests SDK, cibles Nx typecheck/lint/build et format/lint
des fichiers TS/scénario passent. Un test interactif reste ignoré dans Cargo ;
la preuve navigateur est exécutée séparément. Fixture arrêtée et conteneur
supprimé. Pas de coût d'infrastructure nouveau. Notifications effectivement
délivrées, interfaces, politique opérateur et autres exigences A à D restent ouverts.

## Preuve navigateur HTTPS de récupération MFA — 2026-09-10

Le scénario runtime-browser-recovery.mjs traverse Identity et Account réels
avec le SDK actif, les migrations PostgreSQL et des authentificateurs CTAP2
virtuels. Il inscrit une clé, produit une preuve forte, génère les codes et
obtient un premier accès Account OAuth/DPoP. La clé virtuelle est ensuite retirée.
La consommation d'un code pose le cookie de récupération isolé et invalide
l'accès Account déjà délivré. Une annulation native ne termine pas le parcours.

Le remplacement vérifie une vraie attestation, efface les cookies de session
et de récupération puis exige une nouvelle connexion. Une assertion avec la
nouvelle passkey permet de constater le retrait de l'ancienne clé, de générer
un nouveau lot de codes et de retrouver Account avec un nouvel échange OIDC.
Les codes restent en mémoire de page ; aucun secret n'est renvoyé dans le rapport.

Validation Chromium 152.0.7977.83 réussie sur une fixture HTTPS neuve ; seule la
partie authentificateur est virtuelle. Le premier essai a exposé une assertion
de test incorrecte : le SDK encapsule AbortError dans WebauthnBrowserError.
L'assertion vérifie désormais la cause native sans changer le SDK. La fixture
est arrêtée après validation. Format/lint du scénario passent. Aucun Rust ou
SDK n'est modifié ; les trois binaires compilent dans la fixture, sans relance
des suites unitaires déjà validées. Reprise après rechargement, abandon explicite,
notifications délivrées, interfaces et autres exigences A à D restent ouverts.

## SDK de récupération MFA — 2026-09-10

Les fonctions et méthodes hébergées génèrent les dix codes, consomment un code
avec CSRF de session puis inscrivent la passkey de remplacement avec la preuve
de récupération distincte. Le SDK valide les réponses et exige une reconnexion
explicite en fin de parcours. Il ne stocke ni codes ni preuve de récupération.
L'ancien appel générique de génération vers `/auth/mfa/recovery-codes` est
supprimé avec migration documentée. Les autres méthodes MFA génériques restent
à migrer, ainsi que reprise/abandon, preuve HTTPS, interfaces et notifications.

La revalidation Rust compile sans avertissement. La première exécution PostgreSQL
a échoué sur 65 tests OAuth existants, notamment avec Protocol(AccessDenied).
Un test isolé puis la suite complète passent sans changement Rust : 183 tests
bibliothèque, 24 runtime et trois gardes ; un test interactif ignoré. Cause
transitoire non établie : la cohérence temporelle DB/processus reste à vérifier
dans les gates de résilience, sans assouplir les contrôles d'authentification.

Validation SDK : 116 tests sur 22 fichiers passent, ainsi que les cibles Nx
typecheck, lint et build. Les six nouveaux tests remplacent les deux simulations
de l'ancien endpoint de génération. Aucun parcours navigateur de récupération
n'est revendiqué par cette tranche et aucune infrastructure n'est ajoutée.

## Surface HTTP de récupération MFA — 2026-09-10

Les routes génération/consommation et inscription de la passkey de remplacement
sont montées avec les opérations OAuth hébergées. La consommation pose un cookie
HttpOnly de récupération distinct et efface celui de session normale ; le jeton
reste absent du JSON. Sa preuve CSRF est liée à l'origine et au cookie navigateur
avec un domaine HMAC distinct. Le succès final confirme la nécessité de se
reconnecter et efface les cookies, sans créer de session ordinaire.

Le décodeur objet strict de TOTP est extrait et partagé. Les corps sont bornés,
les quotas source précèdent la vérification navigateur et les quotas compte
restent communs entre sessions et avec les opérations MFA/WebAuthn. Une panne
de persistance ne produit pas de cookie de succès. Le contrat liste précisément
les réponses et les opérations restantes côté interface.

Les tests HTTP/PostgreSQL traversent une vraie attestation, couvrent séparation
des preuves, refus Origin/CSRF, ambiguïtés JSON, quotas conservés après remplacement
de session et panne du store. Le SDK, la preuve navigateur HTTPS, les interfaces,
les notifications délivrées et les autres exigences A à D restent ouverts.

## Noyau transactionnel de récupération MFA — 2026-09-10

La migration 0022 et les modules mfa_recovery implémentent génération de dix
codes hachés, consommation après connexion récente, jeton de récupération
séparé d'OAuth et cérémonie de remplacement par une passkey. Le
[contrat](identity-mfa-recovery.md) décrit la séparation des preuves, le besoin
de reconnexion et les limites restant avant exposition HTTP.

Une preuve forte récente protège la génération. La consommation révoque les
sessions ordinaires ; seule la nouvelle attestation vérifiée permet de retirer
les anciens facteurs, effacer le TOTP et invalider les codes restants. Toutes
les mutations concernées et les audits/intentions de notification sont atomiques.
Le remplacement fonctionne aussi avec dix anciennes passkeys. Aucun AMR ou
step-up n'est attribué au code ou au jeton de récupération.

Validation : cargo check --workspace, 176 tests bibliothèque Identity, 24 tests
runtime et trois gardes PostgreSQL passent (un test interactif ignoré). Les sept
nouveaux tests utilisent de vraies signatures, couvrent concurrence, refus OAuth,
isolation, expiration, preuve d'utilisation de la clé récupérée et rollback.

Cette tranche est interne : routes HTTP, cookies/CSRF dédiés, quotas, SDK,
interface et notification réellement délivrée restent à raccorder. Aucun test
navigateur n'est revendiqué pour ce nouveau parcours et aucun service public
n'est ouvert. Les lots A à D restent actifs.

## Compatibilité WebAuthn non découvrable via session authentifiée — 2026-09-10

Le SDK expose loginWithSecurityKey et loginHostedSecurityKey : mot de passe,
puis assertion WebAuthn avec UV obligatoire et allowCredentials obtenu derrière
la session authentifiée. Le choix suit la prévention de fuite des credential IDs
décrite par le [W3C](https://www.w3.org/TR/webauthn-3/#sctn-credential-id-privacy).
Une erreur ou annulation tente le logout et ne retourne pas d'interaction de
succès ; l'erreur distingue explicitement une révocation confirmée d'un cleanup
non confirmé. Le login mot de passe et le step-up sont deux opérations serveur,
pas une nouvelle politique MFA obligatoire.

La preuve Chromium utilise une clé CTAP2 USB virtuelle avec hasResidentKey=false,
sans modifier les options serveur. Elle vérifie inscription, annulation avec
session révoquée, connexion suivie de step-up et accès Account OAuth/DPoP. Les
tests SDK couvrent l'ordre des appels, le refus du mot de passe, l'annulation,
le rejet d'assertion et la panne du logout. Les checks Nx test/typecheck/lint/build
et format/lint des fichiers touchés passent. Aucun Rust modifié ; les trois
binaires compilent dans la fixture. Pas de nouvelle infrastructure.

Le parcours passwordless identifiant puis clé non découvrable n'est pas déclaré
livré ; sa politique de confidentialité reste à trancher si ce besoin est retenu.
Les clés physiques/mobile, récupération MFA, politique opérateur, interfaces,
logout intersites et gates d'exploitation restent ouverts dans les lots A à D.

## Gestion du facteur TOTP et révocation des sessions — 2026-09-10

Les routes hébergées TOTP factors/list et factors/revoke sont raccordées au SDK.
La liste expose uniquement les métadonnées actives du propriétaire. Révoquer
exige une authentification forte récente et une passkey active en remplacement.
La politique de gestion est extraite et partagée avec WebAuthn sans changer
ses exigences. Le verrou principal sérialise les suppressions de facteurs.

La révocation efface le ciphertext, révoque toutes les sessions du principal
et écrit l'audit atomiquement. La rotation MFA exclut désormais les facteurs
révoqués pour ne pas tenter de déchiffrer un secret effacé ; un test vérifie
qu'elle continue à traiter les autres facteurs. Le contrat détaille le besoin
de reconnexion et le refus du dernier facteur avec HTTP 409.

Chromium avec services HTTPS réels vérifie liste, conflit du dernier facteur,
ajout d'une passkey virtuelle et révocation TOTP, puis refus de la session et
de son access token Account. La fixture est arrêtée après cette preuve. Les
tests PostgreSQL couvrent ownership, fraîcheur, suspension, concurrence entre
TOTP/passkey et rollback en cas d'échec d'audit. Le SDK passe ses 106 tests
ainsi que lint/typecheck/build ; aucun coût d'infrastructure supplémentaire.
`cargo check --workspace` et la suite Nx Identity PostgreSQL passent : 169 tests
bibliothèque, 24 runtime et trois gardes de base ; un test interactif ignoré.

La récupération MFA, les anciennes méthodes génériques du SDK, les interfaces,
notifications, politique opérateur et gates d'exploitation restent ouverts.
Cette tranche ne clôture pas les lots A à D.

## SDK et preuve navigateur TOTP — 2026-09-10

Le SDK expose start/confirm/step-up TOTP sur les routes hébergées actives, avec
CSRF de session, transport borné et absence de retry. Le provisioning valide
UUID, secret Base32, cohérence de l'URI avec le profil serveur et expiration.
Les réponses sont projetées sur le contrat public ; les erreurs ne recopient
pas le secret. Les anciennes méthodes et fonctions setup/confirm sur `/auth/*`
sont supprimées, avec migration documentée. Les sept anciens tests simulant
ces endpoints sont remplacés par cinq tests des contrats actifs et une preuve
navigateur réelle.

Chromium HTTPS vérifie enrôlement, confirmation, step-up et refus du rejeu des
deux codes consommés, puis consentement, ID token vérifié et API Account DPoP.
Seul l'authentificateur TOTP est synthétique (WebCrypto HMAC), sans modification
de l'horloge serveur. Le test utilise la fenêtre +1 documentée pour le second
code. Identity, Account, Billing et les migrations sont réels dans la fixture.

Validation : 102 tests SDK, lint/typecheck/build Nx et format/lint des fichiers
touchés réussis. La fixture est arrêtée et son conteneur supprimé. Aucun Rust
modifié : pas de relance Cargo globale ; les trois binaires compilent dans la
fixture. Le parcours navigateur reste interactif, hors CI.

La gestion/récupération MFA et le step-up générique historique restent à migrer.
Les interfaces, politique opérateur, notifications, logout proactif et gates
d'exploitation restent ouverts. Aucun lot A à D n'est clos.

## Enrôlement TOTP lié à la session — 2026-09-10

Les routes `/oauth/session/totp/enrollment/start|confirm` utilisent le cookie et
CSRF de session, des corps JSON objet bornés à 4096 octets et les quotas actifs.
Le nouveau [contrat](identity-totp-enrollment.md) décrit les erreurs, le secret
de provisioning, la conservation bornée et les limites restantes.

La migration 0021 lie le pending à la session et à une expiration de cinq minutes.
Le secret est chiffré avec les primitives actives. Un redémarrage invalide l'ancien
pending ; un facteur actif n'est jamais remplacé. Confirmation, consommation du
compteur, step-up et audits sont atomiques. La politique d'authentification fraîche
est extraite et partagée avec WebAuthn, sans changer sa sémantique.

Validation : `cargo check --workspace` sans avertissement, suite Nx PostgreSQL
Identity (164 tests bibliothèque, 24 runtime, trois gardes ; un test interactif
ignoré). Les neuf nouveaux tests couvrent les routes et la persistance, dont
concurrence et rollback. Ils ont détecté l'acceptation de tableaux par Serde ;
les routes imposent désormais des objets avec refus des doublons et champs inconnus.
Le parcours Chromium WebAuthn complet passe après extraction de la politique.

SDK/interface TOTP, gestion/récupération MFA, notifications, politique opérateur,
logout intersites et gates d'exploitation restent ouverts. Aucun lot A à D n'est
déclaré complet, aucune ouverture publique ou infrastructure nouvelle n'est livrée.

## Remplacement des anciens appels WebAuthn du client — 2026-09-10

`NvbesIdentityWeb` expose désormais register/login/step-up/list/rename/revoke
Passkey en utilisant les fonctions hébergées validées. Les anciennes méthodes
start/finish et le module `mfa.webauthn.ts` sont supprimés : ils appelaient les
routes archivées avec des contrats factor/challenge et un Bearer optionnel.
La recherche des consommateurs actifs ne trouve que le SDK et ses tests.
Le README donne la migration explicite pour d'éventuels consommateurs externes.

Validation : 104 tests SDK, lint/typecheck/build et parcours Chromium HTTPS
réussis. Le parcours WebAuthn utilise maintenant les six méthodes du client,
avec authentificateurs virtuels, vrais services et révocation session/API.
Les deux anciens tests simulant `/auth/mfa/webauthn/start` sont remplacés par
les tests des fonctions hébergées et cette preuve navigateur. Aucun Rust modifié ;
pas de relance Cargo complète, compilation des trois binaires dans la fixture.

La migration TOTP/récupération et step-up générique reste ouverte, ainsi que les
interfaces, logout intersites et gates d'exploitation. Les lots A à D restent actifs.

## Gestion des passkeys via le SDK — 2026-09-10

Le SDK expose liste, renommage et révocation sur les routes Identity actives,
avec CSRF de session, origine exacte et transport borné sans retry. La liste
JSON est validée (dix clés maximum, identifiants distincts, labels et dates),
puis projetée sur les seules métadonnées publiques. Les mutations exigent un
accusé positif ; les erreurs HTTP, dont le conflit du dernier facteur, remontent
au client. Le transport accepte désormais une valeur JSON inconnue pour cette
liste ; les autres endpoints gardent leur validation d'objet obligatoire.

Validation : 106 tests SDK, lint/typecheck/build et parcours Chromium HTTPS
réussis. Le scénario WebAuthn utilise deux authentificateurs CTAP2 virtuels et
prouve renommage, protection du dernier facteur, ajout d'une deuxième clé et
révocation de la première avec refus de sa session et de son jeton Account.
Le parcours multisite Account/Billing est rejoué avec succès après extraction
du transport JSON. Aucun Rust modifié ; pas de relance de la suite Cargo complète.
Les trois binaires sont recompilés par la fixture et les migrations réelles passent.

Restent le remplacement des anciennes méthodes du wrapper, les parcours TOTP
et récupération, les interfaces produit, le logout proactif et les gates
d'exploitation. Cette tranche ne clôture aucun lot A à D à elle seule.

## WebAuthn hébergé dans le SDK — 2026-09-10

Les fonctions dédiées du SDK enregistrent une passkey, connectent un utilisateur
sans identifiant et réalisent un step-up sur les routes actives. Elles valident
les options JSON, le RP exact de l'hôte Identity et UV required, puis utilisent
les conversions et appels natifs existants. Les cérémonies restent liées aux
preuves CSRF d'interaction/session ; une annulation ne déclenche aucun finish.
Les labels respectent la limite serveur de 128 caractères Unicode.

Le parcours Chromium HTTPS réel a révélé que toutes les requêtes WebAuthn
consommaient le quota TOTP de cinq tentatives : un enregistrement et deux step-up
étaient impossibles en dix minutes. La catégorie WebAuthn admet désormais vingt
requêtes par principal dans cette fenêtre, avec les quotas source conservés.
La migration 0020 préserve les anciens compteurs ; TOTP garde ses cinq tentatives.
Le test HTTP/PostgreSQL vérifie saturation, erreurs comptées et budgets distincts.

Validation : 100 tests SDK, lint/typecheck/build, `cargo check --workspace`,
155 tests de bibliothèque Identity, 24 du runtime et trois gardes de base réussis
(un test navigateur interactif ignoré dans Cargo). Chromium vérifie via le SDK
l'enregistrement, deux step-up, une connexion découvrable après logout, puis
OIDC/DPoP/Account. Le scénario multisite Account/Billing passe aussi. La clé CTAP2
est virtuelle ; aucune compatibilité matérielle ou mobile n'est déduite.

Le SDK historique de gestion des facteurs, récupération MFA, interfaces produit,
logout proactif intersites et gates d'exploitation restent ouverts. Les lots A à D
ne sont pas déclarés terminés et aucune ouverture publique n'est effectuée.

## Client SDK des opérations hébergées — 2026-09-10

Le SDK expose le chargement/validation d'interaction, login mot de passe,
consentement et logout sur les routes actives. Le transport exige l'origine
Identity courante, des cookies same-origin et une preuve CSRF explicite ; il
refuse les redirections automatiques, borne les réponses à 64 Kio/dix secondes
et n'effectue aucun retry. Les erreurs de parsing ne recopient pas le corps.
Le résultat de login remplace la preuve d'interaction ; le logout utilise la
preuve distincte de session et exige `logged_out: true`.

L'ancien wrapper logout appelait `/auth/logout` et ignorait le statut HTTP.
Il appelle maintenant `/oauth/logout` avec une preuve explicite et échoue si le
serveur refuse. Les anciennes méthodes MFA/WebAuthn du wrapper restent à migrer.

Validation : 93 tests SDK, lint/typecheck, build du package et parcours Chromium HTTPS des deux
clients réussis. Le banc navigateur utilise désormais ces fonctions pour
login, consentement et logout, au lieu de ses propres appels fetch. Aucun code
Rust modifié ; les compilations des trois binaires passent dans la fixture.
Les interfaces produit et les autres exigences A à D restent ouvertes.

## Parcours HTTPS de deux clients dans Chromium — 2026-09-10

Le SDK réel passe le parcours PAR → login/consentement → callback → refresh →
Account/Billing depuis deux origines clientes distinctes, avec deux clés DPoP
indépendantes. IndexedDB conserve les clés pendant la navigation intersites ;
les ID tokens sont vérifiés, les transactions consommées et les cookies Identity
isolés. Chromium applique le refus CORS pour une origine non enregistrée.
Après logout, les deux API et les refresh sont refusés.

La [preuve et sa procédure](identity-browser-protocol-proof.md) décrivent les
assertions et limites : certificat local accepté dans le seul contexte Chromium,
certificat vérifié par les clients Rust via une autorité éphémère, pages de test
sans UI produit, exécution interactive hors CI. Les trois binaires et migrations
sont réels ; aucune réponse OAuth, introspection ou autorisation n'est simulée.

Validation : Chromium 152.0.7977.83, lint/format des trois modules JavaScript.
Aucun changement Rust dans cette tranche. Les interfaces, récupération MFA,
logout intersites proactif et gates d'exploitation restent ouverts.

## Navigation après consentement hébergé — 2026-09-10

La préparation du parcours navigateur a identifié un écart : approve/deny
attendaient un POST JSON mais renvoyaient toujours 303. Un fetch en mode manual
ne fournit pas Location pour une navigation cliente ; suivre automatiquement
la redirection ne navigue pas la page et traverse la frontière CORS.

Les deux routes acceptent désormais explicitement `Accept: application/json`
pour retourner une destination JSON, puis laisser l'interface Identity effectuer
la navigation de premier niveau. Les appels existants et l'autorisation silencieuse
gardent la redirection 303. La destination reste construite à partir du registre
et de l'interaction serveur, jamais du corps soumis au consentement. Les réponses
restent non cachables, sans referrer, avec Vary Accept.

Les tests couvrent acceptation/refus, encodage du callback, state, rejet
Origin/CSRF et rejeu après consommation. Le scénario des trois services utilise
JSON pour le SDK et conserve les redirections pour les échanges directs.
Cette correction prépare l'interface ; elle ne constitue pas encore la preuve
du parcours navigateur HTTPS ni la livraison des sites.

Validation : `cargo check --workspace`, suite Nx PostgreSQL Identity (154 tests
librairie, 24 runtime et 3 gardes ; un test navigateur interactif ignoré), cible
des trois services et 17 tests DPoP, lint/format JavaScript réussis.

## CORS des API Account et Billing — 2026-09-10

Les services acceptent chacun une liste explicite `NVBES_*_BROWSER_ORIGINS_JSON`,
vide par défaut, validée au démarrage : 32 origines, 16 Kio, HTTPS canonique,
HTTP loopback seulement en développement/test. Pas de normalisation implicite,
d'alias automatique, de wildcard ou de cookie interorigines. La primitive
`core::security::resource_cors` est indépendante de l'ancien AppConfig et de
sa politique de cookies. Les routes utilisateur exposent les erreurs et headers
DPoP ; les routes internes, opérateur, webhook, health et métriques sont exclues.

Le scénario des trois binaires contrôle les preflights, les origines refusées,
les réponses 401 et l'absence de CORS sur les routes exclues. Les appels avec
jetons valides, révoqués ou de mauvaise audience vérifient également le header
d'origine. Aucune infrastructure supplémentaire ; la liste est chargée une seule
fois, sans appel réseau. Le parcours navigateur HTTPS reste à démontrer.

Validation : `cargo check --workspace`, check Billing tous targets, test ciblé
core (la bibliothèque n'a pas de cible Nx), suites Nx PostgreSQL Account
(5 tests et 5 gardes) et Billing (10 tests et 3 gardes), intégration des trois
services (17 tests DPoP et scénario réseau), lint/format JavaScript réussis.

## CORS des endpoints Identity — 2026-09-10

PAR, token et UserInfo autorisent les origines exactes dérivées des redirections
des clients enregistrés. Aucun wildcard ni cookie interorigines ; les headers
Content-Type, Authorization et DPoP sont autorisés, WWW-Authenticate et DPoP-Nonce
sont exposés. Discovery et JWKS sont publics sans credentials. Les routes de
session, login et introspection gardent leur politique sans CORS.

Le test réseau des trois services vérifie les preflights, les origines refusées,
les métadonnées publiques et les erreurs protocolaires qui restent lisibles par
le client autorisé. Il conserve les scénarios SDK, rotation, révocation et panne.
Le test unitaire vérifie aussi le refus d'un domaine suffixé, d'un changement de
schéma et de l'origine opaque `null`, ainsi que Vary et l'exposition du nonce.

Validation : `cargo check --workspace`, cible Nx `test:database` (152 tests
librairie, 24 tests runtime et 3 gardes de base de test ; un test navigateur
interactif ignoré), cible `test:resource-runtimes` (17 tests DPoP et scénario des
trois binaires) et lint/format des deux fichiers JavaScript réussis.

CORS des API Account/Billing, parcours navigateur HTTPS et interfaces restent
à livrer. Ces tests HTTP ne démontrent pas l'application des politiques par un
navigateur. Cette tranche réutilise tower-http déjà présent dans le workspace,
sans infrastructure ni dépendance réseau d'exploitation supplémentaire.

## Validation des ID tokens côté SDK — 2026-09-10

Le callback vérifie désormais la signature RS256 avec `jose` (6.2.9, version
déjà présente dans le lockfile, déclarée comme dépendance directe). Profil du
service actif : type JWT, issuer exact, audience client unique, `azp` cohérent
s'il existe, nonce transactionnel, sujet non vide, expiration et dates entières,
`auth_time` non futur par rapport à l'émission, liaison `at_hash` à l'access token.
Le résultat contient une identité vérifiée distincte du jeton brut.

Le refresh d'une session issue du callback revérifie chaque nouvel ID token et
conserve sujet, client, issuer, nonce et heure d'authentification. Les JWKS sont
chargés à l'adresse fixe de l'issuer configuré, sans URL issue des claims ou
headers JWT, sans redirection/cookie/cache implicite : 10 secondes, 64 Kio,
32 clés maximum. Une panne de clés n'accepte pas les claims sans signature.
Référence : [validation OIDC](https://openid.net/specs/openid-connect-core-1_0.html#IDTokenValidation).

Validation : 86 tests SDK, lint/typecheck et lockfile frozen/offline réussis.
Les tests incluent signatures falsifiées, types et clés inconnus, mauvais
issuer/client/nonce/dates/hash, clé indisponible ou réponse trop grande et
changement de sujet au refresh. Le scénario des trois vrais services passe avec
ces vérifications au callback et pendant les deux rotations. Aucun code Rust
modifié ; les 17 tests DPoP et les compilations sont rejoués par cette cible.

Le parcours navigateur HTTPS, la rotation opérationnelle des clés, les interfaces,
la récupération MFA, le logout intersites et les gates charge/FinOps restent à
livrer ou démontrer. Ce profil local ne revendique aucune certification OIDC/FAPI.

## Refresh SDK et intégration aux trois services — 2026-09-10

`OAuthSession` conserve les jetons en mémoire avec la clé DPoP issue du callback.
Un seul échange est lancé pour les appels de refresh simultanés ; chaque rotation
utilise le nouveau secret, conserve la clé et génère un nouveau `jti`. Une réponse
Bearer, une extension de scopes, une rotation absente ou une durée invalide sont
refusées. Une erreur invalide la session locale sans nouvelle tentative : le
serveur peut avoir déjà consommé le secret. Délai réseau de dix secondes ; aucun
cookie ni redirection. Une réponse tardive ne réactive pas une session effacée.

La cible des vrais runtimes charge désormais le SDK TypeScript via tsx avec son
tsconfig explicite. Pour Account puis Billing : PAR signé par le SDK, interaction
hébergée réelle, échange de code SDK, refresh concurrent mutualisé, seconde
rotation, appel métier DPoP accepté et refresh refusé après logout. Les migrations
et données sont dans les trois bases du conteneur éphémère habituel. Le stockage
de clé est injecté en mémoire dans Node ; aucune validation CORS/HTTPS navigateur
n'est déduite de ce scénario.

Validation : 81 tests SDK, lint et typecheck réussis ; les 17 tests DPoP Rust,
compilations des trois binaires et scénario réel étendu passent. Les tests couvrent
aussi panne transport, downgrade, scope modifié et effacement pendant le refresh.
Aucun code Rust modifié ; la suite PostgreSQL Identity complète n'est pas relancée.
Restent notamment validation OIDC des ID tokens, parcours navigateur HTTPS,
interfaces hébergées, récupération MFA, logout intersites et gates FinOps.

## Clé DPoP du SDK à travers la redirection — 2026-09-10

Le SDK utilise DPoP par défaut avec une clé WebCrypto non exportable par
transaction. IndexedDB conserve la clé par structured clone, jamais sous forme
de JWK privé ; 32 entrées maximum, durée des transactions de quinze minutes,
nettoyage des entrées expirées à l'admission. PAR et token signent des preuves
distinctes avec cette même clé. Le callback contrôle la référence, le thumbprint,
l'issuer, le client et la redirection ; une clé manquante ou une réponse Bearer
sont refusées. Après succès, la clé revient en mémoire dans `dpopKey` pour les
requêtes ressources explicites et l'entrée temporaire est supprimée.

Validation : 77 tests SDK, lint et typecheck réussis. Contrôle Playwright avec
Chromium réel sur HTTP loopback : rechargement de page, même clé récupérée,
signature ES256 vérifiée, refus d'export, saturation à 32 entrées, suppression.
Un second scénario traverse PAR puis token après rechargement avec sessionStorage
et IndexedDB réels, mais réponses OAuth simulées ; mêmes clés et `jti` distincts,
états temporaires effacés. Les bases et le serveur Vite temporaires ont été retirés.

Les tests automatisés vérifient aussi absence de clé, changement de contexte,
downgrade, échec PAR et préservation d'une transaction déjà ouverte. Aucun test
Rust relancé : aucun code Rust touché. Restent les échanges avec les vrais
services sous HTTPS, refresh DPoP, validation des ID tokens et les autres lots.

## Paramètres PAR du SDK — 2026-09-10

La configuration publique du SDK utilise désormais `resource`, URL exacte du
registre Identity, au lieu de l'ancien `audience`. Les requêtes exigent `openid`,
un nonce et un state ASCII de 16 à 512 caractères. Le README utilise les scopes
actifs Account. Les origines Identity non sûres et les chemins de retour ambigus
sont refusés avant sauvegarde de transaction et appel réseau. PAR refuse les
redirections et son résultat doit contenir un URI de requête au préfixe attendu.

Validation : 73 tests SDK réussis, typecheck et lint ciblés. Ces tests examinent
les formulaires émis avec un transport simulé ; ils ne constituent pas un parcours
navigateur avec le serveur. Le stockage de clé DPoP et le raccordement complet
PAR/token/refresh/API restent ouverts, ainsi que les autres lots A à D.

## Primitive DPoP navigateur — 2026-09-10

Suppression du fallback réseau de `dpopFetch` : un échec crypto n'envoie aucune
requête, un échec transport n'entraîne aucune répétition implicite. Les requêtes
DPoP omettent les cookies et refusent les redirections. Les clés privées WebCrypto
ne sont plus exportables ; les nonces sont isolés par origine avec cache borné.
Les preuves normalisent `htu` sans query ni fragment.

Les tests vérifient le refus d'export, l'isolation des nonces et l'absence de
fallback après erreur de signature ou de transport. Les 72 tests SDK passent,
ainsi que lint et typecheck ciblés. Le chemin des types OpenAPI est explicite
dans le tsconfig du SDK et une option de suppression de dépréciation incompatible
avec le compilateur installé a été retirée.

Le parcours OAuth utilise encore son ancien contrat `audience` et des scopes
à réaligner. La clé DPoP est encore en mémoire : stockage non exportable au
retour de redirection, raccordement PAR/token/refresh/API et vrais tests navigateur
restent à réaliser. Aucun parcours complet n'est revendiqué par cette tranche.

## Header DPoP sur PAR — 2026-09-10

Le endpoint PAR accepte une preuve ES256 liée à POST et à l'URL de l'issuer
configuré. Sa clé est injectée avant validation de la politique du client ;
un `dpop_jkt` simultané différent ou des headers dupliqués sont refusés.
La consommation anti-rejeu et la création PAR partagent une transaction.
Un échec de stockage permet de réessayer la même preuve sans état partiel.
Les deux mécanismes correspondent au contrat de la
[RFC 9449, section 10.1](https://www.rfc-editor.org/rfc/rfc9449.html#section-10.1).

Validation : `cargo check --workspace` réussi ; la cible Nx
`identity-service:test:database` passe avec 151 tests de bibliothèque et
24 tests runtime, plus trois contrôles de base isolée. Le test navigateur
interactif reste ignoré. Les nouveaux tests traversent HTTP et PostgreSQL
pour la liaison de clé, le mismatch, les doublons, la concurrence et le rollback.
Le SDK navigateur et les parcours multisites restent à livrer.

## DPoP dans les API ressources — 2026-09-10

Account et Billing acceptent maintenant les tokens liés à DPoP avec preuve
ES256, tout en interdisant leur consommation en Bearer. La primitive partagée
vérifie la liaison cryptographique à la clé, la méthode, l'URL publique configurée,
la date et le hash du jeton. Elle refuse aussi les clés privées et courbes
incohérentes dans les preuves, y compris sur les endpoints Identity existants.

Chaque API conserve les marqueurs anti-rejeu dans sa propre base (migration
0002), avec verrouillage partagé entre répliques, 64 compartiments et au plus
256 lignes par compartiment. Le nettoyage a lieu à la consommation, sans Redis
ni tâche récurrente. Les saturations et pannes refusent l'accès. Le
[contrat](identity-resource-dpop.md) précise les horloges, limites et variables.

La preuve réelle des trois runtimes couvre émission Identity, erreurs de liaison,
rejeu concurrent, redémarrage, panne du store, saturation de 16 384 lignes,
nettoyage des places expirées et logout. Les preuves sont signées par Node avec
une clé P-256 éphémère. Aucun déploiement, paiement réel ou trafic externe.

Validation : 17 tests de la primitive DPoP, 173 tests Identity avec PostgreSQL
(un test navigateur interactif ignoré), cinq tests Account et dix tests Billing
avec PostgreSQL, compilation workspace et check Billing réussis. Le test réel
des trois runtimes passe également. SDK navigateur, header DPoP de PAR, logout
intersites, interfaces hébergées et gates charge/FinOps restent ouverts.

## Autorisation des comptes facturés — 2026-09-10

Les handlers Billing consultent désormais l'autorité Account après validation
et introspection Identity. Les scopes ne suffisent plus : le demandeur doit
posséder le profil personnel actif ou être le propriétaire cohérent d'une équipe
active. Les refus interviennent avant les lectures Billing et effets Stripe.
Les routes typées distinguent `principal` et `team` ; les alias `workspaces`
restent exclusivement des équipes. Les lectures SQL filtrent aussi le type.

Le [contrat](billing-account-authorization.md) décrit l'authentification serveur
dédiée, les limites et les refus en cas de panne. Le helper local génère un
secret atomique privé distinct d'Identity ; aucune configuration de production
n'est écrite. La décision Account n'est jamais mise en cache côté Billing.

La validation réelle des trois runtimes est étendue aux tiers, membres, types
incompatibles, changements de rôle et fermetures avec les mêmes JWT, ainsi qu'à
la panne/reprise d'Account. Les refus ne créent ni customer, ni checkout, ni
audit Billing. Les opérations autorisées utilisent le provider dummy local.
Les tests PostgreSQL Account et adversariaux du client Billing complètent cette
preuve. Les lots A à D restent ouverts, dont DPoP, logout intersites, frontend
hébergé et gates de charge/FinOps.

Validation : `cargo check --workspace`, check Billing toutes cibles, cinq tests
Account avec PostgreSQL, dix tests Billing avec PostgreSQL, parcours des trois
runtimes, deux tests du helper local, lint/format et syntaxe Bash réussis.

## Preuve entre les trois runtimes — 2026-09-10

La cible Nx `identity-service:test:resource-runtimes` compile et lance les vrais
binaires Identity, Account et Billing. Elle crée un conteneur PostgreSQL local
éphémère limité à 256 Mio, trois bases distinctes avec leurs migrations réelles,
une paire RSA et deux secrets de ressource aléatoires. Les environnements des
services ne reprennent aucune configuration de production ni télémétrie. Le
compte synthétique est créé par la commande existante, sans envoi d'email.

Le test effectue les échanges HTTP hébergés de login, consentement et Code/PKCE.
Il vérifie les lectures métier Account/Billing avec des jetons réellement émis
par Identity, le refus des audiences croisées, puis le refus des deux jetons
après POST `/oauth/logout`. Une nouvelle connexion fonctionne ; l'arrêt du
processus Identity entraîne 503 sur les deux API, puis son redémarrage rétablit
l'accès aux seules sessions actives. Les anciennes sessions restent révoquées.

Validation : cible Nx réussie, scénario complet en environ sept secondes hors
compilation. Le nettoyage arrête les processus et retire le conteneur dédié,
y compris lors des échecs. L'image `postgres:17-alpine` doit être disponible
localement ; aucun téléchargement implicite ni abonnement n'est ajouté. Les
ports sont temporaires sur loopback ; Identity utilise `localhost` pour son RP
WebAuthn, qui refuse une adresse IP comme identifiant de domaine.

La gestion des cookies est explicite dans Node : cette preuve ne couvre pas les
politiques d'un navigateur sous HTTPS, DPoP, la rotation des clés ou le logout
intersites. Aucun paiement, déploiement ni ouverture publique n'est effectué.
La lecture des handlers Billing a également identifié un contrôle à traiter :
les scopes sont vérifiés, mais les routes `/workspaces/{id}/billing/*` ne relient
pas encore cet identifiant au principal ou à son rôle Account. La preuve de
transport/introspection n'atteste donc pas l'isolation métier entre comptes ;
cette autorisation reste un préalable à toute exposition du service.

## Consommation de l'introspection par Account — 2026-09-10

Account utilise le même client SDK que Billing après validation locale du JWT.
L'extracteur refuse les credentials HTTP ambigus et les tokens inactifs ; une
panne d'introspection produit 503 avant le handler. Le JWT doit respecter
l'issuer exact, l'audience canonique Account, les scopes, les identifiants et
les bornes temporelles du profil actif. Les tokens liés à DPoP sont refusés en
Bearer. La configuration ne retire plus le slash final de l'issuer.

Le contrôle des actions sensibles conserve une échéance d'authentification forte
et la vérifie au moment de l'autorisation, après la consultation d'Identity.
Un AMR historique sans preuve temporelle valide ne suffit plus pour un export
ou une fermeture. L'échéance est plafonnée par l'expiration du token ; une
connexion primaire passkey est recevable pendant cinq minutes.

Les credentials de ressource Account sont obligatoires et distincts de Billing.
Le helper local publie deux secrets persistants et un registre cohérent, sans
remplacer un registre personnalisé. Le graphe Nx déclare la dépendance au SDK
et le cache du check inclut ses sources. Aucune configuration de production
n'est écrite et aucun déploiement n'est effectué.

Les nouveaux tests utilisent des JWT signés et une fixture HTTP contrôlée pour
vérifier scopes/audiences/dates, activité et révocation, panne, headers dupliqués,
refus avant le handler sensible et expiration des preuves. Le parcours avec
les vrais runtimes Identity, Account et Billing demeure une validation à mener,
ainsi que DPoP, le logout intersites et les gates globaux.

Validation : `cargo check --workspace`, Nx `account-service:check` toutes cibles,
les quatre tests de `account-service:test:database` sur une base neuve isolée,
les deux tests du helper local, son lint/format et la syntaxe Bash passent.
Le fournisseur d'introspection utilisé dans le test HTTP Account est simulé ;
ces résultats ne constituent pas encore une preuve interservices complète.

## Consommation de l'introspection par Billing — 2026-09-10

L'extracteur Billing vérifie désormais le JWT puis consulte Identity. Une
réponse inactive donne 401 ; une panne, un timeout ou une réponse incohérente
donne 503 avant l'entrée dans le handler métier. La validation locale seule
n'est plus une voie d'authentification du runtime. Les tests unitaires peuvent
encore l'exercer isolément, dans une fonction compilée uniquement pour les tests.

Le client est dans le SDK Rust existant afin de le réutiliser pour Account.
Il borne les consultations à 16 simultanées, 2,5 secondes (connexion une seconde)
et 16 Kio de réponse. Aucun retry ni cache positif ; pas de redirection ni proxy
implicite. Le header Basic est marqué sensible. Une réponse active doit retrouver
exactement les claims attendues du JWT vérifié localement ; une autre session,
audience, scope ou date ne peut pas être substituée par la réponse HTTP.

Les cinq paramètres Identity de Billing doivent être fournis ensemble. Le
helper de développement génère et publie atomiquement un secret de ressource
en mode 0600 ; plusieurs services qui démarrent simultanément lisent la même
valeur. Le registre local et Billing reçoivent des valeurs cohérentes sans
sortie contenant le secret. Un registre personnalisé n'est pas remplacé : son
propriétaire fournit les credentials correspondants. Les audiences par défaut
du helper et d'Account sont alignées sur les audiences canoniques du protocole.
Cela ne provisionne pas un registre OAuth public ni une configuration de production.

Validation : compilation workspace et check Billing toutes cibles réussis.
Les 19 tests du SDK (unitaires et intégration existante), les huit tests Billing
avec PostgreSQL et les deux tests du helper local passent. Les nouveaux tests
HTTP couvrent activité/révocation simulée sans cache, claims substituées, réponse
malformée/trop grande, redirection, erreurs, timeout, saturation et rejet avant
le handler Billing. Le fournisseur HTTP est une fixture locale contrôlée ; le
parcours réunissant Identity réel et Billing reste une validation à effectuer,
ainsi que le branchement Account, DPoP et les gates de charge/FinOps.

## Endpoint d'introspection authentifié — 2026-09-10

Identity monte POST `/oauth/introspect` lorsque le registre des ressources
`NVBES_IDENTITY_RESOURCE_SERVERS_JSON` est configuré avec le registre OAuth.
HTTP Basic authentifie chaque API avec une clé de 32 octets distincte, comparée
avec la primitive à temps constant de `subtle`. L'audience de consultation est
imposée par le registre ; un service Billing ne peut pas introspecter un token
Account. Aucun credential n'est enregistré dans les logs ni fourni par défaut.

La requête est un formulaire borné avec un token et un hint facultatif ignoré.
Les doublons, paramètres inconnus et query sont refusés. La réponse inactive
ne révèle aucune claim. Une consultation valide relit l'état actuel de grant,
session, principal et client. Les erreurs de store et le délai de deux secondes
retournent 503 ; les réponses demandent `no-store`. Le quota protocolaire source
existant s'applique. Le [contrat détaillé](identity-token-lifecycle.md) documente
les limites et la configuration, sans revendiquer le branchement des API clientes.

Validation : `cargo check --workspace` passe sans avertissement. La cible Nx
PostgreSQL Identity passe avec 149 tests bibliothèque et 24 tests runtime ; le
test navigateur interactif reste ignoré. Les nouveaux scénarios couvrent les
credentials incorrects/ambigus, la mauvaise audience, les grants et sessions
révoqués, les formulaires invalides, une table indisponible et un verrou qui
dépasse le délai, suivi d'une requête réussie après libération. Aucun nouveau
parcours navigateur ni déploiement n'est effectué.

Les consommateurs Account/Billing, leur comportement en cas de panne et la
validation des preuves DPoP restent les étapes suivantes du lot C/D.

## Contrat des jetons d'accès Billing — 2026-09-10

L'intégration de révocation a révélé que le vérificateur Billing acceptait un
bearer arbitraire sans clé publique et ignorait l'audience avec une clé. Ce
contournement est supprimé. Le service exige le profil RS256 `at+jwt`, la clé
configurée, l'émetteur exact, l'audience `nvbes-billing-service`, des identifiants
de session/grant/principal valides et une durée maximale de 900 secondes.
Les scopes acceptés sont `billing:read` et `billing:checkout` ; le checkout utilise
maintenant ce dernier, conformément au profil Identity. Le pseudo-scope
`billing:admin` ne donne plus de passe-droit.

La configuration emploie le triplet canonique de clé publique, issuer et key ID.
Sans triplet, les API utilisateur sont indisponibles à l'authentification, sans
empêcher les webhooks Stripe de fonctionner. Un triplet partiel est une erreur
de démarrage. Le helper de développement exporte déjà ces trois variables.
Le signal d'authentification forte tient compte de la fraîcheur de la preuve,
pas seulement de la présence historique de `totp` ou `webauthn` dans `amr`.

Cette correction est un préalable au contrôle de révocation ; l'appel à Identity
depuis Account/Billing n'est pas encore implémenté. DPoP n'est pas encore accepté
par Billing : un token contenant une confirmation de clé est refusé en Bearer.

Validation : `cargo check --workspace`, Nx `billing-service:check` (toutes les
cibles Cargo) et `billing-service:test` passent. Les tests de sécurité utilisent
des JWT signés avec une paire RSA éphémère ; aucune clé de test n'est stockée.
La cible PostgreSQL `billing-service:test:database` passe également sur une base
neuve isolée `nvbes_billing_test_identity`, avec Stripe simulé localement.
Les tests vérifient le cycle Billing et le rollback/rejeu des webhooks sans
recourir à l'ancien contournement bearer. Aucun appel de paiement réel ni
déploiement n'est effectué.

## Révocation des sessions liées aux passkeys — 2026-09-10

La révocation d'un credential invalide maintenant, dans la même transaction,
toutes les sessions auxquelles il a contribué (connexion primaire ou step-up).
La session appelante est également révoquée si elle a utilisé cette clé ; elle
doit alors se reconnecter avant une nouvelle opération de gestion. L'enrollment
d'une autre clé n'efface pas cette provenance. Une session du même compte qui
n'a pas utilisé la clé reste active.

Les lectures transactionnelles OAuth, MFA et WebAuthn prennent désormais le
verrou du principal avant celui des sessions. Les échanges de code, chargements
de grants, logout et récupération suivent le même ordre pour éviter le cycle
entre une révocation multi-session et une opération détenant déjà une session.
Les vérifications d'activité sont exécutées après acquisition du verrou.

L'échange d'un code en attente, l'introspection, UserInfo et le refresh utilisent
le contrôle d'activité de la session. Révoquer celle-ci rend donc ses grants
inutilisables dans ces chemins sans devoir réécrire chaque ligne de jeton.
La validation cryptographique locale d'un JWT ne consulte pas cet état : la
propagation aux API Account/Billing et leur politique de cache restent ouvertes.

La migration 0019 révoque les sessions créées avant la migration d'attribution
0018, les sessions WebAuthn sans association correspondant à leur rôle et celles
liées à une clé déjà révoquée. Même une ancienne session affichant maintenant
TOTP peut avoir utilisé WebAuthn auparavant : aucune provenance n'est supposée.
Cette transition produit un audit par compte avec le nombre de sessions révoquées.
Elle impose une reconnexion aux anciennes sessions lors de son déploiement ;
aucun déploiement n'est effectué ici.

Les scénarios PostgreSQL couvrent le login passkey primaire, le step-up, la
session appelante, une session indépendante préservée, le code non échangé,
l'introspection et le refresh après révocation, le rollback d'audit et la
transition des anciennes sessions. La validation utilise la base isolée
`nvbes_identity_test_revocation` sur le conteneur de tests dédié (port 15433).
L'ancienne base de développement du test de migration n'est pas réutilisée
après modification de la migration non publiée.

Validation finale : `cargo check --workspace` sans avertissement et cible Nx
`identity-service:test:database` réussie (143 tests de bibliothèque, 24 runtime,
un test navigateur interactif ignoré). Aucun parcours navigateur supplémentaire
n'a été exécuté pour cette évolution de persistance et de contrôle des sessions.

## Attribution des sessions aux credentials — 2026-09-10

La migration 0018 conserve les clés ayant contribué à une session dans
`identity_session_webauthn_credentials`. Chaque association précise le
propriétaire et le rôle primaire ou step-up. Les clés étrangères composites
interdisent une association entre comptes distincts. Un index par credential
prépare la sélection des sessions concernées par une révocation.

La connexion passkey et le step-up écrivent cette association dans leur
transaction existante, après vérification cryptographique. Une authentification
répétée avec la même clé actualise sa date ; une autre clé ajoute une association
sans effacer la première. Une panne ultérieure d'audit ou de liaison OAuth annule
aussi l'association. L'enrollment seul ne constitue pas une authentification
et n'écrit donc pas d'association.

Cette étape fournit l'attribution nécessaire au lot C ; elle ne propage pas
encore la révocation aux sessions ou jetons. Les sessions antérieures à cette
migration n'ont pas d'attribution fiable : aucune clé n'est inventée à partir
de leur méthode `webauthn`. La politique de transition devra invalider ces
sessions sans attribution avant de revendiquer une révocation complète.

Validation : compilation workspace réussie ; cible Nx PostgreSQL avec 139 tests
de bibliothèque et 24 tests runtime réussis, un test navigateur interactif ignoré.
Les assertions vérifient l'association primaire, le rollback après panne OAuth
ou audit, le refus des associations entre comptes, l'absence de doublon et la
conservation de deux clés utilisées depuis deux authentificateurs logiciels.
Le parcours navigateur n'a pas été rejoué pour cette modification de persistance.

## Gestion des credentials WebAuthn — 2026-09-10

POST `/oauth/session/webauthn/credentials/list`, `/rename` et `/revoke` sont
montés derrière les protections Origin/CSRF de session, les quotas source/MFA
et la limite JSON existante. List reçoit `{}` et retourne au plus dix clés
actives du propriétaire avec `id`, `label`, `created_at`, `last_used_at`.
Aucun identifiant cryptographique brut, Passkey sérialisé ou clé publique n'est
retourné par ce contrat de gestion.

Rename reçoit `{credential_id,label}` et revoke `{credential_id}`. Les mutations
exigent une session active avec WebAuthn primaire récent ou step-up TOTP/WebAuthn
récent (cinq minutes). Le propriétaire et le credential sont verrouillés ; la
mutation et l'audit sont atomiques. Un renommage identique et une nouvelle
révocation d'une clé déjà révoquée du propriétaire ne dupliquent pas l'audit.

Révoquer la dernière passkey utilisable nécessite un autre credential actif
avec état complet ou un TOTP actif. Une configuration TOTP encore pending et
une ancienne ligne sans Passkey complet ne suffisent pas. Le refus HTTP est
409 `{error: "last_strong_factor"}`. Le verrou de principal sérialise aussi
les révocations concurrentes depuis deux sessions pour conserver cette propriété.

La révocation interdit les assertions futures, y compris les cérémonies déjà
commencées. Les sessions préexistantes suivent encore leur cycle distinct :
la propagation de révocation aux sessions/jetons, les notifications et la
récupération restent des exigences ouvertes, notamment du lot C. Les routes
livrées ici ne constituent donc pas une clôture des lots B/C ni de l'objectif A–D.

Les tests utilisent des enrollments/assertions signés et PostgreSQL. Ils
couvrent propriétaires distincts authentifiés, fraîcheur, labels invalides,
idempotence, autre facteur pending/actif, concurrence entre sessions, panne
d'audit avec rollback et routes HTTP (mauvaise origine, métadonnées et dernier
facteur). La liste reste accessible avec une session active sans step-up récent.

Validation : `cargo check --workspace` passe sans avertissement. La cible Nx
`identity-service:test:database` passe avec 137 tests de bibliothèque et 24 tests
du runtime ; le test navigateur interactif reste ignoré dans cette suite. Aucun
nouveau parcours navigateur n'a été exécuté pour ces routes de gestion, couvertes
ici par les tests HTTP avec PostgreSQL et assertions WebAuthn signées.

## Parcours Chromium contre les vraies API Identity — 2026-09-10

Le [harnais Rust](../../apps/identity-service/src/identity.browser.e2e_fixture.rs)
monte les routeurs réels sur un port loopback aléatoire, dans un schéma PostgreSQL
isolé et avec un compte synthétique. Il est compilé uniquement sous
`cfg(test)` avec `database-tests`, jamais dans le runtime de production. La cible
`identity-service:test:browser-fixture` lance ce test interactif explicitement ;
le test est ignoré par la suite automatique standard car il attend un navigateur.

Le [probe Playwright](../../apps/identity-service/tests/webauthn-api.playwright.js)
reçoit l'origine affichée par le harnais. Dans un contexte Chromium neuf avec
authentificateur CTAP2 virtuel, il appelle les vraies API, sans interception de
requêtes : autorisation, login mot de passe, enrollment via `credentials.create`,
logout, nouvelle autorisation, login passkey via `credentials.get` sans liste
de credentials, consentement, échange Code/PKCE et lecture de UserInfo. Le
navigateur produit lui-même les réponses WebAuthn et le userHandle. Aucun
identifiant de credential n'est injecté dans la phase de login.

Cette validation a révélé que `TokenConfig::from_values` refusait encore
`nvbes-identity-userinfo`, bien que les tests internes ajoutent cette audience
après parsing. Le parseur est corrigé avec un test d'acceptation exacte et de
refus d'un nom voisin ; le harnais utilise maintenant ce chemin de configuration.

Résultat observé : Chromium 152.0.7977.83 valide enrollment, login sans identifiant,
échange du code et claims UserInfo. Le test Rust vérifie ensuite une session
passkey active et un grant avec jeton émis, supprime son schéma et termine en
succès. Compilation workspace sans avertissement ; suite PostgreSQL : 131 tests
de bibliothèque, 24 tests du runtime et un test interactif ignoré par défaut.
Ce dernier a été exécuté séparément et passe. Le script Playwright passe lint
et format ; les secrets temporaires ne sont pas affichés dans les résultats.

Limites : authentificateur virtuel et HTTP loopback en mode développement,
authentification modale explicite. Ceci ne valide pas encore l'autofill
conditional UI, HTTPS et navigation entre plusieurs sites en production,
Firefox/Safari, clés physiques, apps natives, charge ni les autres lots A–D.

## Préférence de découvrabilité et preuve Chromium — 2026-09-10

L'enrollment standard de webauthn-rs émet `residentKey: discouraged`. Le probe
[Playwright](../../apps/identity-service/tests/webauthn-residency.playwright.js)
a confirmé avec Chromium 152.0.7977.83 et un authentificateur CTAP2 virtuel que
cette option produit un credential non découvrable. Avec `preferred`, le même
authentificateur crée un credential découvrable, puis `credentials.get` avec
`allowCredentials: []` retourne une assertion et son userHandle sans sélection
de credential injectée par le test.

L'enrollment actif demande donc désormais `residentKey: preferred`, avec le type
de `webauthn-rs-proto`. `requireResidentKey` reste faux et `userVerification`
reste obligatoire. Cette préférence accepte toujours les credentials classiques,
en cohérence avec la politique d'acceptation conservée dans l'état de cérémonie.
Elle ne modifie ni challenge ni exigence cryptographique et n'est pas une preuve
de résidence : [WebAuthn](https://www.w3.org/TR/webauthn-3/) autorise encore un
credential non découvrable avec cette préférence. Aucun signal n'est promu en
garantie matérielle ou en niveau d'assurance.

Validation : compilation workspace sans avertissement ; cible Nx
`identity-service:test:database` avec 130 tests de bibliothèque et 24 tests du
runtime réussis. Le test d'enrollment vérifie les trois options et conserve les
tests cryptographiques de refus en absence d'UV. Le probe Chromium a été exécuté
dans un contexte neuf, fermé à la fin, sans comptes utilisateur ni clé réelle.

Limite : le probe isole les options WebAuthn dans une page de test interceptée,
sans appeler le backend. Il n'est pas une preuve E2E Identity ni un test de l'UX
conditional UI. L'enchaînement réel navigateur/API, les autres navigateurs et
le login avec identifiant pour les clés non découvrables restent à terminer.

## Routes HTTP du login primaire passkey — 2026-09-10

Le runtime hébergé monte POST `/oauth/authorize/passkey/options` et
`/oauth/authorize/passkey/finish`, avec le même RP, registre client et origine
navigateur que les autres routes Identity. Les deux routes portent les quotas
de source protocole/login, la validation Origin/cookies/CSRF navigateur et une
limite de corps de 65536 octets. Le moteur revalide ensuite le CSRF exact de
l'interaction et le registre avant de créer ou consommer une cérémonie.

Options reçoit `{interaction}` et retourne `{ceremony_id, options}`. Finish
reçoit `{interaction, ceremony_id, credential}` et retourne l'interaction, son
nouveau `csrf_token` et le `session_csrf_token`. La session opaque sort seulement
dans le cookie hôte HttpOnly/Secure/SameSite=Lax, après commit. Cette construction
de réponse est partagée avec le login par mot de passe. Les erreurs de parsing
ou de stockage ne reflètent pas les credentials et ne posent pas de cookie.

Le login sans identifiant utilise les quotas de source, sans compter un
`userHandle` non authentifié dans le quota d'un compte cible. La limitation
globale en charge et les scénarios de panne restent des gates de livraison.

Validation : compilation workspace sans avertissement et cible Nx
`identity-service:test:database` avec 130 tests de bibliothèque et 24 tests du
runtime réussis. Les nouveaux tests HTTP produisent une assertion signée,
vérifient le cookie et poursuivent jusqu'au consentement avec redirection 303.
Ils couvrent aussi origine/CSRF incorrects, gros corps, quota source, rejeu et
panne SQL sans cookie, suivie du retry de la même assertion.

Le comportement graphique de conditional UI et l'enrollment de credentials
découvrables restent à valider avec un navigateur. Le simulateur utilise la
sélection décrite dans la tranche précédente, sans démontrer cette UX. Restent
également la gestion/récupération des facteurs et les autres exigences A–D.

## Login primaire passkey lié à OAuth — 2026-09-10

`oauth::passkey_login` démarre une cérémonie discoverable sans identifiant de
compte, après validation du navigateur, du CSRF et de l'interaction OAuth.
La migration 0017 relie la cérémonie à la requête OAuth avec suppression en
cascade et unicité ; un nouveau départ remplace la cérémonie précédente.

À la fin, `userHandle` est traité comme un indice non authentifié : le principal
actif et son credential non révoqué sont verrouillés puis la bibliothèque
vérifie l'assertion avec le credential courant et l'UV obligatoire. Elle vérifie
donc aussi le compteur courant, sans utiliser une ancienne copie de clé.
La mise à jour du Passkey, la consommation de la cérémonie, la création d'une
session `primary_amr=webauthn`, l'audit et le rattachement OAuth avec rotation
CSRF partagent une transaction. Aucun token de session ne sort avant commit.

Validation : `cargo check --workspace` sans avertissement ; cible Nx
`identity-service:test:database` avec 127 tests de bibliothèque et 24 tests du
runtime réussis. Les tests couvrent les signatures valides jusqu'au code OAuth,
le rejeu, les substitutions navigateur/compte, deux finishes concurrents, une
clé révoquée, un principal suspendu et la panne du rattachement OAuth. Cette
panne ne laisse ni session passkey ni mise à jour du credential ; la même
assertion peut être réessayée après restauration.

Limite de preuve : SoftPasskey ne possède pas d'UI de sélection discoverable.
Le test lui fournit le credential sélectionné et le userHandle, tandis que
l'état conservé côté serveur reste celui du parcours discoverable. Cela vérifie
les signatures et la liaison de compte, pas le comportement d'un navigateur.
Les routes HTTP du login primaire, l'UX conditional UI, l'enrollment assurant
la disponibilité de credentials découvrables et le parcours des clés non
découvrables restent à terminer. Les lots A–D ne sont pas clos.

## Routes HTTP d'enrollment et step-up WebAuthn — 2026-09-10

Le runtime monte désormais les quatre routes POST sous
`/oauth/session/webauthn/` : `registration/options`, `registration/finish`,
`step-up/options` et `step-up/finish`. Elles sont activées avec le protocole
hébergé et `NVBES_IDENTITY_BROWSER_ORIGIN`. Le RP ID est exactement le hostname
de cette origine, sans élargissement aux domaines parents ou sous-domaines.

Ordre de protection : quota source TCP (protocole puis login), Origin/cookies
et CSRF lié à la session, vérification de session active et quota MFA du
principal, puis parsing JSON et opérations transactionnelles. Les quatre routes
partagent le quota MFA existant avec TOTP, y compris options et tentatives
invalides. Les corps sont bornés à 65536 octets ; les options exigent `{}` et
les corps de finalisation refusent les champs inconnus. Les erreurs ne reflètent
pas les détails SQL ni les credentials et toutes les réponses sont `no-store`.

Contrats JSON : les options retournent `ceremony_id` et `options` à transmettre
à l'API WebAuthn du navigateur. Registration finish reçoit `ceremony_id`,
`credential` et `label`, puis retourne l'UUID interne `credential_id`. Step-up
finish reçoit `ceremony_id` et `credential`, puis retourne `step_up` et
`expires_at`. Le header `x-csrf-token` utilise le `session_csrf_token` obtenu
dans le parcours d'autorisation hébergée, avec les cookies hôte Identity.

Les tests HTTP/PostgreSQL font un enrollment puis un step-up avec un
authentificateur logiciel et vérifient le rejeu. Ils couvrent les quatre routes
contre mauvaise origine/CSRF et quota source, les corps invalides et trop gros,
le quota de compte persistant après refus et la panne du stockage des quotas.
Un test a révélé que Serde acceptait `[]` comme struct vide : les options exigent
maintenant une map JSON vide. Le parcours navigateur graphique, le login
primaire passkey et la gestion/récupération des facteurs restent à livrer.

Validation : `cargo check --workspace` sans avertissement et cible Nx
`identity-service:test:database` avec 122 tests de bibliothèque et 24 tests du
runtime réussis. Aucun navigateur graphique n'a été exécuté pour cette tranche.

## Step-up WebAuthn transactionnel — 2026-09-10

`step_up::start` crée une cérémonie de cinq minutes liée à une session active,
avec les passkeys non révoquées de son principal. `step_up::finish` vérifie
l'assertion et l'UV contre l'état serveur, relit le credential sous verrou,
contrôle son compteur courant puis met à jour le Passkey, consomme le challenge
et accorde le step-up dans une seule transaction avec audit. La preuve récente
dure au plus dix minutes et ne dépasse pas l'expiration de la session.

La migration 0016 ajoute la révocation individuelle des credentials. Une clé
révoquée après le début de la cérémonie est refusée à la fin. Les anciens
credentials sans Passkey complet restent exclus. Les limites d'enrollment et
la présence d'un facteur existant portent désormais sur les clés non révoquées.

L'option `danger-credential-internals` de webauthn-rs est utilisée uniquement
pour lire le compteur typé du credential courant sous verrou, sans reconstruire
ni affaiblir la politique cryptographique. La vérification initiale utilise une
copie datant du début de cérémonie : ce second contrôle refuse une assertion
devenue obsolète après l'utilisation de la même clé dans une autre session.
La mise à jour reste faite par `Passkey::update_credential`.

Validation : compilation workspace sans avertissement et cible Nx
`identity-service:test:database` avec 117 tests de bibliothèque et 24 tests du
runtime réussis. Six tests PostgreSQL exercent les assertions signées, le rejeu,
la concurrence, la révocation, une autre session, l'expiration, une signature
altérée, les compteurs entre sessions et une panne d'écriture de session. La
panne annule aussi la mise à jour du credential et autorise la même preuve au retry.

Ce module interne n'est pas encore un parcours HTTP livré. Restent les adapters
Origin/CSRF/quotas, le login primaire passkey, la gestion des credentials,
la récupération et les parcours navigateur. Les lots A–D restent ouverts.

## Enrollment WebAuthn transactionnel — 2026-09-10

Les anciens helpers publics qui séparaient challenge aléatoire, état de
cérémonie, vérification et persistance ont été remplacés par `registration::start`
et `registration::finish`. Le challenge stocké est exactement celui généré par
webauthn-rs, avec son état complet, son principal et sa session. Une nouvelle
demande remplace la précédente pour cette session ; la durée est de cinq minutes.

Le début et la fin exigent une session active et une authentification récente.
Si le compte a déjà une passkey ou un MFA actif, une preuve récente WebAuthn ou
un step-up TOTP/WebAuthn est exigé. Le principal est verrouillé pour sérialiser
les ajouts depuis plusieurs sessions et borner les credentials à dix par compte.
La fin vérifie la réponse contre l'état serveur, puis écrit le Passkey complet,
consomme la cérémonie et audite dans une transaction unique.

La migration 0015 ajoute la liaison de session et le credential sérialisé.
Les anciennes lignes ne contenant qu'une clé publique ne sont pas transformées
en Passkey par supposition : elles nécessitent un nouvel enrollment. Le signal
discoverable reste inconnu (`NULL`) au lieu d'être affirmé par défaut. Le Passkey
complet est la source des compteurs, flags et règles de vérification ; les
anciennes colonnes isolées ne suffisent pas à authentifier.

Les tests PostgreSQL utilisent un authentificateur logiciel de la bibliothèque,
exclusivement en dépendance de développement. Ils produisent de vraies réponses
signées et vérifient une assertion après relecture du credential. Ils couvrent
origine incorrecte, absence d'UV, substitution de challenge, autre session,
expiration, révocation, rejeu, concurrence, facteur existant et panne d'écriture.
L'authentificateur de test simule l'UV ; il ne démontre aucune propriété matérielle.

Validation : `cargo check --workspace` sans avertissement ; cible Nx
`identity-service:test:database` avec 111 tests de bibliothèque et 24 tests du
runtime réussis, sur PostgreSQL isolé avec la migration 0015.

Cette tranche ne monte pas encore de routes WebAuthn : l'adaptateur HTTP doit
vérifier Origin et CSRF de session et appliquer les quotas avant ces opérations.
Restent le login/step-up transactionnel, les credentials découvrables, les
routes d'enrollment et de gestion, la récupération et les parcours navigateur.
Elle ne clôt donc pas le lot B ni les lots A–D.

## UserInfo et construction des endpoints — 2026-09-10

UserInfo dispose d'une audience dédiée, exige `openid` et ne retourne l'email
que si le scope est accordé et l'identifiant actuellement vérifié. Les jetons
Account/Billing et les ID tokens sont refusés. La session, le grant et la
politique client sont relus à chaque appel. Le traitement DPoP vérifie la clé,
la méthode, l'URL et `ath`, puis consomme la preuve dans la transaction de
lecture des claims. Un échec SQL annule cette consommation.

Les tests HTTP/PostgreSQL couvrent GET/POST, scopes, suspension, révocation,
réduction de politique, transport des credentials, headers dupliqués, preuves
invalides, rejeu concurrent et panne SQL avec nouvelle tentative de même preuve.
Les fixtures UserInfo utilisent des schémas privés pour isoler la panne injectée.
La construction des endpoints corrige aussi un défaut avec les issuers sans
slash final : discovery et cibles DPoP ont désormais un séparateur explicite,
sans modifier le claim `iss` configuré.

Validation : `cargo check --workspace` passe sans avertissement ; la cible Nx
`identity-service:test:database` passe avec 106 tests de bibliothèque et 24 tests
du runtime. Les parcours SDK/navigateur, WebAuthn et les autres exigences A–D
restent ouverts. Les échecs intermittents précédemment observés restent à
investiguer ; cette exécution verte ne démontre pas leur résolution.

## Échange de code atomique — 2026-09-10

POST `/oauth/token` utilise désormais `TokenService::exchange_code` : validation
cryptographique de la preuve DPoP sur l'URL canonique du service, puis une seule
transaction pour la consommation du code, le grant, le marqueur anti-rejeu,
la signature, le refresh éventuel et les audits. Un échec de signature ou
d'insertion du refresh annule l'ensemble. Aucun jeton n'est retourné avant commit.

Le rejeu authentifié d'un code consommé reste un résultat métier distinct :
la révocation et la consommation de sa nouvelle preuve sont committées avant
de retourner `invalid_grant`. Si la preuve elle-même a déjà été utilisée, la
transaction annule la tentative de révocation et son audit. Deux requêtes
identiques n'invalident donc pas le gagnant ; deux preuves fraîches sur le même
code déclenchent bien la révocation attendue.

Validation : `cargo check --workspace` sans avertissement ; la dernière cible
Nx `identity-service:test:database` passe avec 97 tests de bibliothèque, 24 tests
du runtime et 3 contrôles de base isolée. Les six nouveaux tests passent via le
routeur HTTP et PostgreSQL, dont une panne SQL après signature. Une première
exécution avait produit 12 échecs dans des tests de session/autorisation existants ;
la relance et l'exécution finale passent. Leur cause intermittente reste à
investiguer dans la validation globale, sans l'attribuer à l'environnement faute
de preuve. UserInfo et les parcours SDK/navigateur restent à terminer.

## Autorisation directe et PAR — 2026-09-10

Le décodage HTTP conserve les paires avant validation pour refuser les paramètres
dupliqués, y compris leurs noms encodés. `max_age` est converti explicitement en
entier et conserve la limite de 86400 secondes. La query directe et le corps PAR
sont bornés à 8192 octets et 32 paramètres. Un paramètre `request` non pris en
charge est refusé, sans repli silencieux vers un autre parcours.

PAR renvoie 201 avec une référence opaque. Sa rédemption accepte seulement
`client_id` et `request_uri`, sans paramètres capables de modifier la requête
enregistrée. La consommation du PAR, la création de l'unique interaction et sa
liaison navigateur/CSRF/session se font dans une même transaction PostgreSQL.
Un navigateur invalide ne consomme rien ; un échec de liaison annule aussi la
consommation et permet de réessayer. La création directe utilise la même frontière
transactionnelle. Une panne de consentement en mode silencieux renvoie 503,
sans redirection mensongère `login_required`.

Les tests HTTP/PostgreSQL couvrent les doublons, `max_age`, les substitutions,
le rejeu, la concurrence entre navigateurs, la panne injectée et le retour arrière.
Cette tranche ne valide pas encore un parcours SDK/navigateur jusqu'aux API
Account/Billing. UserInfo reste à terminer ; l'échange de code atomique est
documenté dans la tranche ci-dessus.

Validation : `cargo check --workspace` sans avertissement et
`identity-service:test:database` avec 91 tests de bibliothèque, 24 tests du runtime
et 3 contrôles de base isolée réussis. Aucun test graphique de navigateur n'est
inclus dans cette tranche.

## Quotas HTTP — 2026-09-10

Les routeurs OAuth exigent désormais un limiteur à clé stable ; le runtime exige
`NVBES_IDENTITY_RATE_LIMIT_KEY` lorsque le registre public est configuré et fournit
l'adresse TCP via `ConnectInfo`. Les en-têtes proxy ne sont pas utilisés.
Toutes les routes du protocole portent le quota de source avant parsing ; login
et TOTP ont un quota de source plus strict et un quota distinct par compte.
Les tentatives restent comptées après un refus métier. La migration 0014 étend
la contrainte des catégories à MFA sans effacer les compteurs existants.

HTTP 429 comporte `Retry-After` et `no-store`. Une panne ou un délai de quota
supérieur à deux secondes renvoie 503 ; il n'existe pas de fallback permissif.
Les tests couvrent les routes réelles, les headers proxy falsifiés, la clé,
la normalisation email/IPv6, la panne du stockage, l'expiration de fenêtre et
le partage du quota MFA entre sessions. Voir les limites et la configuration
d'exploitation dans [le cycle des jetons](identity-token-lifecycle.md#quotas-des-routes-http).
Les parcours navigateur, la topologie proxy, la charge et le budget restent
des gates de livraison ; aucune ouverture publique n'est effectuée.

Validation : `cargo check --workspace` sans avertissement ; la cible Nx
`identity-service:test:database` passe avec 85 tests de bibliothèque, 24 tests
du runtime et 3 contrôles de base isolée. Le test TCP utilise un vrai serveur
Axum sur loopback ; les autres tests HTTP traversent les routeurs et PostgreSQL.

## Calculs de mot de passe et annulation — 2026-09-10

La vérification Argon2, la vérification factice et le renouvellement du hash
conservent désormais leur permis de concurrence dans le travail bloquant.
L'annulation de la requête HTTP ne libère donc pas de capacité tant que le calcul
continue. Les tests pilotent explicitement un calcul en cours, annulent son
appelant puis vérifient que le permis reste occupé jusqu'à sa fin. Une panique
du calcul libère aussi le permis. Les quotas HTTP sont raccordés dans la tranche
documentée ci-dessus.

Validation : `cargo check --workspace` sans avertissement ;
`identity-service:test:database` passe avec 77 tests de bibliothèque, 24 tests
du runtime et 3 contrôles de base isolée. Les deux nouveaux tests de capacité
s'exécutent dans les deux cibles Rust qui compilent le module d'authentification.

## Login hébergé atomique — 2026-09-10

Le login contrôle l'interaction et son CSRF avant le calcul de mot de passe.
Argon2 et le calcul éventuel du nouveau hash s'exécutent sans transaction ouverte.
La transaction finale revérifie l'interaction, le registre, le statut du principal,
l'identifiant et le hash du mot de passe. Elle crée la session, écrit l'audit,
lie l'interaction et renouvelle le CSRF avant un unique commit.

Les tests HTTP/PostgreSQL vérifient l'absence de session orpheline sur mauvais
CSRF, les soumissions concurrentes et la permutation de session affichée. Un
trigger d'échec dans le schéma de test prouve le retour arrière de la session,
de l'audit et du CSRF lors d'un échec de liaison, puis la possibilité de réessayer.
Un autre test modifie les identifiants ou suspend le principal après vérification
du mot de passe et confirme que la création de session est refusée.

Validation : `cargo check --workspace` sans avertissement ; la cible Nx
`identity-service:test:database` passe avec 75 tests de bibliothèque, 22 tests
du runtime et 3 contrôles de base isolée. Ces preuves HTTP/PostgreSQL ne couvrent
pas encore les parcours de navigateur graphique.

## Sessions navigateur — 2026-09-10

Logout et step-up TOTP exigent une preuve CSRF liée par HMAC à la session,
au cookie navigateur et à l'origine. Le login et l'autorisation avec une session
active renvoient `session_csrf_token`, distinct du `csrf_token` de consentement.
Toutes les mutations hébergées passent par le middleware navigateur avant
lecture du JSON ; les deux mutations de session vérifient aussi le HMAC à cet
endroit. Le cookie navigateur est conservé entre autorisations concurrentes.

Le logout révoque et audite dans une transaction, sans audit dupliqué au rejeu.
Le step-up vérifie le statut actif du principal sous verrou. Les scénarios HTTP
avec PostgreSQL couvrent la preuve émise après login, les onglets concurrents,
le mauvais CSRF/origin/session, le refus avant parsing JSON, le code TOTP non
consommé lors d'un refus, le rejeu TOTP et la panne du stockage. Les fixtures
HTTP ont chacune un schéma isolé pour ne pas contaminer les tests de rotation
globale des clés MFA.

Validation : `cargo check --workspace` sans avertissement ;
`identity-service:test:database` passe avec 70 tests de bibliothèque, 22 tests
du runtime et 3 contrôles de base isolée. Les tests exécutent les routes Axum
et PostgreSQL ; aucun parcours de navigateur graphique n'est encore attesté.

Le logout intersites reste à terminer avant ouverture publique. Les limites
HTTP ont été raccordées dans la tranche documentée ci-dessus.

## Renouvellement et DPoP — 2026-09-10

Le refresh HTTP exige désormais le client enregistré et, pour un grant DPoP,
une preuve cryptographique de la même clé. Les rotations et rejeux de générations
différentes prennent les verrous dans le même ordre. La consommation de preuve,
la signature, la rotation et l'audit sont dans une transaction PostgreSQL.
Les headers DPoP dupliqués sont refusés sur le token endpoint ; les réponses de
jetons sont non stockables. Les erreurs de stockage deviennent HTTP 503.

Les tests actifs traversent le routeur token et PostgreSQL : mauvais client,
clé/méthode/URL erronées, preuve absente ou dupliquée, rejeu de preuve, rejeu du
refresh, concurrence, retour arrière après échec de signature, expiration,
révocation de session et suspension de principal. Ils ne prouvent pas encore
un parcours navigateur avec SDK et API de ressource.

Validation de cette tranche : `cargo check --workspace` sans avertissement ;
`identity-service:test:database` passe avec 62 tests de bibliothèque, 22 tests
du runtime et 3 tests du garde de base. Les migrations 0001–0013 sont appliquées
dans un conteneur PostgreSQL 17 dédié sur loopback, sans utiliser les bases
applicatives. Le passage des migrations WebAuthn ne valide pas ses cérémonies.

## Correction HTTP vérifiée au 2026-09-10

Les retours d'autorisation utilisent désormais HTTP 303, avec `Cache-Control:
no-store`, `Pragma: no-cache` et `Referrer-Policy: no-referrer`. Un test réseau
local suit les redirections après acceptation et refus : le callback reçoit
GET et aucun corps du POST initial. Les paramètres enregistrés et l'encodage
de `state` sont vérifiés. Ce test porte sur la construction et le suivi HTTP
de la réponse ; il ne prouve pas le parcours de consentement avec PostgreSQL.

Les handlers de session sont extraits pour maintenir le module HTTP sous 500
lignes. Les fonctions de récupération utilisées par le diagnostic interne
sont séparées de la bibliothèque OAuth, sans changement de logique ni nouvelle
route publique. `cargo check --workspace` passe sans avertissement.
La cible Nx `identity-service:test` passe : 36 tests de bibliothèque et 16 du
binaire, dont les deux nouveaux tests de redirection. Les tests PostgreSQL et
navigateur ne sont pas exécutés dans cette tranche.

Le tableau et les résultats ci-dessous restent un suivi historique partiel :
ils ne constituent pas une validation de bout en bout des lots A à D. Le
raccordement WebAuthn, DPoP dans les SDK/API, UserInfo et les contrôles
de session HTTP restent notamment à terminer et à tester.

## État historique au 2026-09-07, enrichi pendant l'implémentation

Les routes publiques sont montées sous configuration dans le runtime actif.
Leur présence ne vaut pas validation pour une ouverture publique. Les nouvelles
primitives sont dans la bibliothèque Identity, pas dans une application archivée.

| Exigence                                                                                 | État et preuve                                                                                                                                                                    |
| ---------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| A — clients publics explicitement enregistrés, redirections exactes, scopes par audience | Implémenté dans `identity.oauth.clients.rs` et `identity.oauth.request.rs` ; tests de refus redirection/scope/HTTP hors loopback                                                  |
| A — PKCE S256 et refus downgrade                                                         | Implémenté ; vecteur RFC 7636 et valeurs invalides testés                                                                                                                         |
| A — stockage des transactions et codes à usage unique                                    | Migration 0007 ; handles/codes hashés, sessions et principal contrôlés sous verrou                                                                                                |
| A — consommation concurrente, rejeu et révocation de grant                               | Tests PostgreSQL passent ; rejeu valide révoque le grant même après expiration du code                                                                                            |
| A — serveur HTTP autorisation/token, discovery, JWKS, UserInfo et ID tokens              | Discovery/JWKS/token/PAR/authorization, login, consentement et UserInfo sont montés sous configuration ; WebAuthn et UI complète restent à livrer                                 |
| A — consentement, cookies/CSRF/Origin/CORS et throttling                                 | Cookie navigateur et liaison transactionnelle sont montés sur GET authorization ; preuve CSRF/Origin, consentement et limites PostgreSQL restent actifs ; login UI reste à monter |
| A — audience/scopes cohérents Account/Billing, audit et erreurs publiques                | À terminer ; `billing:checkout` de PR 175 diverge du `billing:write` actuel                                                                                                       |
| B — passkeys, clés de sécurité et enrollment                                             | Persistance dédiée des credentials/challenges ajoutée en migration 0012 ; cérémonies webauthn-rs et classification d’assurance restent à implémenter                              |
| B — récupération, facteurs supplémentaires et politique opérateur                        | À implémenter ; ne pas confondre reset mot de passe et récupération MFA                                                                                                           |
| B — step-up frais / AMR exact / TOTP HTTP                                                | Méthode primaire et dates de preuve enregistrées ; endpoint POST step-up TOTP protégé par cookie/CSRF et rejeu de compteur ; WebAuthn et exigences AMR restantes                  |
| C — refresh rotation, familles, concurrence et détection de rejeu                        | Familles PostgreSQL hashées, rotation atomique et grant type HTTP `refresh_token` implémentés ; rejeu révoque famille/grant                                                       |
| C — introspection/révocation, rotation JWKS et logout intersites                         | Introspection du grant/session/principal, rotation de clés et logout HTTP avec révocation de session sont montés ; logout intersites/back-channel restent à livrer                |
| D — PAR HTTP et consommation atomique                                                    | Endpoint POST `/oauth/par` validant puis stockant une requête opaque ; consommation PostgreSQL atomique prête pour le raccordement authorization                                  |
| D — DPoP AS + SDK + serveurs de ressources                                               | Preuve ES256, `htm`/`htu`/iat et thumbprint vérifiés ; consommation anti-rejeu PostgreSQL ajoutée ; raccordement route token et resource servers restant                          |
| D — délégation machine si nécessaire                                                     | À statuer selon façade sécurité ; pas de reprise du Token Exchange Cloud                                                                                                          |
| Gates — contrats, vrais parcours, caches, panne, migration et budget                     | À compléter avant clôture ; aucune ressource payante ajoutée                                                                                                                      |

## Prochaine tranche

1. Monter le login hébergé et les routes Code/PKCE/OIDC sur les primitives de
   navigateur et de consentement désormais disponibles ; tester les erreurs de protocole
   et documenter leurs contrats. Puis poursuivre B, C et D sans réduire le scope.
2. UserInfo doit disposer d'une audience explicitement autorisée : ne pas accepter
   arbitrairement un access token destiné exclusivement à Account ou Billing.
   La prise en charge des scopes OIDC/profile/email doit correspondre aux claims
   réellement servis. Le jkt doit devenir une preuve DPoP vérifiée, pas une chaîne
   déclarée par l'appelant.

Les tables de transactions disposent d'un nettoyage borné avant exposition. Les
timestamps de fraîcheur sont enregistrés et testés, mais leur contrôle dans
Account/Billing reste à implémenter. `prompt=login`/`prompt=none`, le refus
utilisateur et la rotation de la preuve CSRF après login sont couverts par les
primitives ; le renouvellement et la politique de déconnexion restent à livrer.

L'émission a été déplacée dans la bibliothèque du package, sans seconde pile de
signature. Le diagnostic synthétique traverse désormais le vrai grant OAuth.
La migration 0008 ajoute les preuves primaires, les dates de step-up, le snapshot
d'autorisation et la consommation unique de l'émission. La migration 0009 ajoute
la liaison navigateur/CSRF/session des interactions, les consentements exacts et
les buckets de limite fixes. La migration 0010 ajoute les familles de refresh
hashées. Voir le
[cycle des jetons et la procédure de rotation](identity-token-lifecycle.md).

## Validation effectuée

- `cargo check --workspace` : succès dans le worktree isolé.
- Cible Nx `identity-service:test:database` : 40 tests de bibliothèque OAuth,
  navigateur, consentement, limite et tokens, 22 tests du binaire, plus 3 tests
  du garde de base de données réussis.
- Base dédiée `nvbes_identity_test_protocol`, PostgreSQL local sur port 15433 ;
  migrations 0001–0011 appliquées. Aucun test sur les bases applicatives.
- Tests PostgreSQL de concurrence PAR, échange de code et émission ; rejeu
  invalidant un jeton déjà émis ; suspension avant signature ; retrait du client
  et preuves MFA capturées avant une modification ultérieure de la session.
- Tests cryptographiques d'audiences/types distincts, at_hash/nonce/auth_time,
  claims hostiles, clés faibles/non appariées, rotation avec échéance et AMR exact.
- Tests des frontières navigateur : cookie de développement explicitement distinct,
  cookie concurrent ou header dupliqué refusés, origine exacte, métadonnées Fetch,
  CSRF et type JSON requis avant toute mutation. Les consentements sont liés à
  l'interaction, à la session affichée et à une politique exacte ; concurrence,
  session révoquée, registre modifié, consentement expiré et mode silencieux sont
  couverts. Les limites utilisent 4096 buckets HMAC par catégorie et leur
  nettoyage ne supprime pas les preuves nécessaires au rejet de rejeu.
- Tests de rotation refresh sur PostgreSQL : un seul secret actif à la fois,
  rotation vers une nouvelle valeur opaque, rejeu de l'ancienne valeur et
  invalidation de la famille et du grant.
- Tests DPoP : signature ES256, méthode/URL incorrectes et consommation unique
  du `jti` pour une même clé sur la base PostgreSQL.
- Cible Nx `identity-service:test:contract` : 11 tests réussis ; ce contrat
  confirme pour l'instant que le runtime public reste fermé.
- Les routes publiques, facteurs WebAuthn, refresh et DPoP complets restent à
  tester après leur implémentation : le vert actuel ne prouve pas les lots A–D.

Pour exécuter Nx dans ce worktree, `node_modules` est un lien local vers les
dépendances du checkout principal ; `pnpm_config_verify_deps_before_run=false`
évite que pnpm tente de réinstaller ce répertoire partagé. Il ne désactive pas
les tests ni les hooks. Les dépendances Rust sont compilées dans le target propre
au worktree. Ne pas committer le lien node_modules ni le document Stripe local.
