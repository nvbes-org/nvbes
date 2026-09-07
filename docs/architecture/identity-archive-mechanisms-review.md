# Inventaire des mécanismes Identity archivés

L'implémentation demandée des lots A à D est suivie dans le
[journal d'exécution](identity-protocol-implementation.work.md).

## Portée et méthode

Relevé statique du 2026-09-07 sur la branche de préparation, runtime de base
`94cba92c`. Inventaire des capacités d'authentification, OAuth, sessions et
protections associées, regroupées par mécanisme ; pas audit de chaque ligne
ni inventaire de toute la sécurité de tous les produits archivés.

Découverte des 564 fichiers Identity backend/web, lecture des routeurs,
métadonnées, modules et implémentations représentatives ; recherche transversale
aux archives pour fédération et extensions. Chaque ligne fournit une source.
« Code » signifie implémentation repérée et examinée statiquement, sans garantie
de compilation, d'intégration, de conformité ou de sécurité complète. Les tests
archivés sont des cas à reprendre, pas des preuves d'exécution actuelle.

Le service actif monte seulement health/métriques : « interne actif » ne signifie
jamais API utilisable par un site. Sources actives :
[main](../../apps/identity-service/src/main.rs),
[auth](../../apps/identity-service/src/identity.auth.rs),
[MFA](../../apps/identity-service/src/identity.mfa.rs),
[tokens](../../apps/identity-service/src/identity.tokens.rs),
[migrations](../../apps/identity-service/migrations),
[core password](../../libs/rust/core/src/auth.helpers.password.rs).

## Inventaire et décisions de réutilisation

Les priorités sont proposées pour le socle B2C. Aucune ligne n'autorise un
copier-coller complet : les contrats, données et tests actifs restent l'autorité.

| Mécanisme / source                                                                                                                               | Preuve archive                                                    | Équivalent actuel                                           | Pertinence et décision                                                                           |
| ------------------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------- | ----------------------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| [Mot de passe, hash/pepper, rehash et vérification factice](../../archive/apps/identity-service/src/identity.domains.auth.password.rs)           | Code ; primitives core déjà partagées                             | Interne actif pour hash/login                               | Conserver core ; adapter HTTP et politique sans recopier                                         |
| [Historique/changement de mot de passe](../../archive/apps/identity-service/src/identity.domains.auth.password.history.rs)                       | Code                                                              | Récupération interne seulement                              | Adapter si mot de passe retenu ; historique ne remplace pas détection de compromission           |
| [Email vérifié et changement d’identifiant](../../archive/apps/identity-service/src/identity.domains.auth.email_verification.change.rs)          | Code                                                              | Email/récupération internes, pas parcours public            | Prioritaire ; séparer identifiant Identity et profil Account                                     |
| [Inscription et enrollment préalable](../../archive/apps/identity-service/src/identity.domains.auth.registration_enrollment.rs)                  | Code                                                              | Création synthétique active                                 | Réécrire sur invitations et tables actives ; ouverture publique hors lot                         |
| [Login par identifiant puis facteurs](../../archive/apps/identity-service/src/identity.domains.auth.routes.login.identifier_flow.rs)             | Code                                                              | Pas de route active                                         | Adapter avec réponse anti-énumération, limites et transactions                                   |
| [Passkeys et login sans identifiant](../../archive/apps/identity-service/src/identity.domains.auth.webauthn.login.discoverable.rs)               | Code avec webauthn-rs et Redis                                    | Absent du runtime Identity actif                            | Prioritaire ; adapter stockage et consommation atomique du challenge                             |
| [Clés de sécurité et enrollment WebAuthn](../../archive/apps/identity-service/src/identity.domains.auth.webauthn.registration.options.rs)        | Code ; UV required et choix platform/cross-platform               | Absent                                                      | Prioritaire ; vérifier cohérence options/état de cérémonie et compatibilité                      |
| [Classification passkey synchronisée / locale / matérielle](../../archive/apps/identity-service/src/identity.domains.auth.webauthn.assurance.rs) | Code ; attribution automatique AAL3                               | Absente                                                     | Réécrire : indicateurs backup et kind ne démontrent pas AAL3                                     |
| [TOTP](../../archive/apps/identity-service/src/identity.domains.auth.mfa.totp.rs)                                                                | Code                                                              | Interne actif, secret chiffré et anti-rejeu                 | Conserver primitive active ; adapter enrollment et routes, pas le stockage archivé               |
| [Codes de récupération MFA](../../archive/apps/identity-service/src/identity.domains.auth.mfa.recovery.rs)                                       | Code                                                              | Pas équivalent MFA attesté ; recovery mot de passe distinct | Prioritaire ; consommation atomique, protection génération et notification                       |
| [Mot de passe oublié/reset](../../archive/apps/identity-service/src/identity.domains.auth.password.reset.rs)                                     | Code                                                              | Primitives internes actives                                 | Adapter l’UX ; expiration, usage unique et révocation de sessions                                |
| [Step-up par mot de passe/TOTP/WebAuthn/recovery](../../archive/apps/identity-service/src/identity.domains.auth.verification.method.rs)          | Code ; email limité au changement de mot de passe                 | TOTP step-up interne actif                                  | Adapter ; ne jamais promouvoir récupération/email en preuve forte                                |
| [Politiques MFA et comptes privilégiés](../../archive/apps/identity-service/src/identity.domains.auth.mfa.policy.rs)                             | Code ; dépendance Cloud et memberships historiques                | Pas politique HTTP attestée                                 | Réécrire frontière Account ; passkey opérateur pertinente                                        |
| [SMS OTP Twilio + provider mock](../../archive/apps/identity-service/src/identity.domains.auth.otp.twilio.rs)                                    | Adaptateur concret ; parcours complet non démontré                | Absent                                                      | Écarter par défaut : coûts variables, dépendance fournisseur, niveau inférieur aux passkeys      |
| [Sessions navigateur et multi-comptes](../../archive/apps/identity-service/src/identity.domains.auth.account_chooser.rs)                         | Code, cookies distincts et Redis                                  | Session opaque interne, aucun cookie HTTP actif             | Adapter plus tard ; isolation cache et logout par compte avant multi-compte                      |
| [Cookies hôte / HttpOnly / Secure et CSRF signé](../../archive/apps/identity-service/src/identity.http.cookies.rs)                               | Code et tests inclus                                              | Pas surface navigateur Identity active                      | Prioritaire ; revoir SameSite=Strict pour vrai parcours intersites                               |
| [CORS et validation Origin](../../archive/apps/identity-service/src/identity.http.cors.origin.rs)                                                | Code                                                              | Non attesté pour futurs parcours                            | Adapter avec liste exacte d’origines ; aucune confiance fondée seulement sur CORS                |
| [Détection de vol de cookie](../../archive/apps/identity-service/src/identity.domains.auth.sessions.cookie_theft.rs)                             | Heuristiques de changement de profil                              | Absente du runtime Identity                                 | Conditionnel ; signal Risk, pas preuve cryptographique ni blocage systématique                   |
| [Device trust / installation reconnue](../../archive/apps/identity-service/src/identity.domains.auth.device_trust.policy.rs)                     | Scores et token d’installation                                    | Absent                                                      | Conditionnel ; ne remplace pas MFA, attestation ou autorisation                                  |
| [Authorization Code + PKCE S256](../../archive/apps/identity-service/src/identity.domains.oauth.validation.pkce.rs)                              | Validation pure et route OAuth archivée                           | Pas endpoint actif                                          | Prioritaire ; adapter types/erreurs et tester code unique/client/redirect                        |
| [État d’autorisation hébergée](../../archive/apps/identity-service/src/identity.domains.oauth.hosted.service.rs)                                 | Code, état expirant et callbacks                                  | Absent                                                      | Prioritaire ; transaction liée à client/PKCE/state/nonce                                         |
| [OIDC discovery, UserInfo, ID token](../../archive/apps/identity-service/src/identity.domains.oauth.metadata.rs)                                 | Code et métadonnées                                               | JWT/JWKS internes seulement                                 | Prioritaire ; ne publier que les capacités effectivement testées                                 |
| [Subjects pairwise](../../archive/apps/identity-service/src/identity.domains.oauth.metadata.rs)                                                  | Annoncé ; émetteur inspecté utilise UUID global                   | Absent                                                      | Ne pas annoncer ; concevoir dérivation sectorielle avant besoin de tiers                         |
| [Audiences, resource indicators et scopes](../../archive/apps/identity-service/src/identity.domains.oauth.validation.target.rs)                  | Code                                                              | Audience Account contrôlée ; Billing divergent              | Prioritaire ; réaligner domaines actifs et tests de refus croisés                                |
| [Consentement utilisateur / administrateur](../../archive/apps/identity-service/src/identity.domains.oauth.consent.rs)                           | Code, tables tenant historiques                                   | Absent                                                      | Adapter consentement utilisateur ; gouvernance Enterprise hors V1                                |
| [Refresh rotation et révocation de famille](../../archive/apps/identity-service/src/identity.domains.oauth.flows.refresh.rs)                     | Code Redis, verrous et dépendance Cloud                           | Absent ; session révocable interne seulement                | Prioritaire si refresh retenu ; réécrire persistance et concurrence sans Cloud                   |
| [Introspection / révocation](../../archive/apps/identity-service/src/identity.domains.oauth.flows.tokens.introspect.rs)                          | Code                                                              | Introspection interne active, aucune route                  | Adapter au contrat service et définir délai de révocation                                        |
| [PAR, requête poussée et consommation](../../archive/apps/identity-service/src/identity.domains.oauth.routes.par.store.rs)                       | Code Redis avec verrou                                            | Absent                                                      | Pertinent pour durcissement ; adapter stockage et tester double consommation                     |
| [JAR, requêtes signées](../../archive/apps/identity-service/src/identity.domains.oauth.jar.high_assurance.rs)                                    | Code et validation de clés/claims                                 | Absent                                                      | Conditionnel ; utile pour clients adaptés, pas secret embarqué en SPA                            |
| [DPoP, nonce et anti-rejeu](../../archive/apps/identity-service/src/identity.http.middleware.dpop.rs)                                            | Middleware + crate partagée nvbes-dpop                            | Non branché dans Identity actif                             | Pertinent ; adapter de bout en bout AS, SDK et resource servers                                  |
| [mTLS et liaison au certificat](../../archive/apps/identity-service/src/identity.http.mtls.rs)                                                   | Acceptor TLS et certificat pair                                   | Absent d’Identity actif                                     | Ciblé machines ; différer pour utilisateurs web, coût opérationnel à prouver                     |
| [Client Credentials et comptes de service](../../archive/apps/identity-service/src/identity.domains.oauth.flows.client_credentials.rs)           | Code, politiques et audit                                         | Pas grant HTTP actif                                        | Adapter si délégation machine nécessaire ; scope minimal, identités séparées                     |
| [private_key_jwt / anti-rejeu assertions](../../archive/apps/identity-service/src/identity.domains.oauth.client_assertion.verify.rs)             | Code, clés et replay store                                        | Absent                                                      | Pertinent pour clients confidentiels, jamais identité secrète d’une SPA                          |
| [Secrets clients et rotation des clés clients](../../archive/apps/identity-service/src/identity.domains.oauth.clients.keys.rs)                   | Code de gestion                                                   | Absent                                                      | Adapter au besoin machine ; réduire surface administrative publique                              |
| [Token Exchange](../../archive/apps/identity-service/src/identity.domains.oauth.flows.token_exchange.rs)                                         | Code limité à nvbes-cloud-service                                 | Absent                                                      | Réécrire pour toute délégation Account/Identity/Billing ; aucune reprise directe                 |
| [Device Authorization](../../archive/apps/identity-service/src/identity.domains.oauth.device.exchange.rs)                                        | Code de polling/échange avec Redis et Cloud                       | Absent                                                      | Garder référence pour TV ; différer tant qu’aucun client de ce type                              |
| [RAR / authorization_details](../../archive/apps/identity-service/src/identity.domains.oauth.rar.rs)                                             | Validation structurelle ; metadata annonce types Drive            | Absent                                                      | Différer ; modèle métier et enforcement requis, JSON validé insuffisant                          |
| [Profil high_assurance / FAPI](../../archive/apps/identity-service/src/identity.domains.oauth.profiles.rs)                                       | Profil local, refuse client public, exige clés et DPoP/mTLS       | Absent                                                      | Pas preuve FAPI ; décision BFF/client confidentiel et conformance avant activation               |
| [ACR / AMR / auth_time / max_age](../../archive/apps/identity-service/src/identity.domains.oauth.assurance.rs)                                   | Code Redis + rôles Cloud                                          | amr et step-up partiels actifs                              | Adapter fraîcheur et niveau démontré ; supprimer dépendances Cloud                               |
| [Backchannel logout](../../archive/apps/identity-service/src/identity.domains.oauth.security_events.rs)                                          | Outbox, signature, envoi HTTP                                     | Audit/outbox internes, pas dispatch protocolaire attesté    | Pertinent pour plusieurs clients ; adapter retry/idempotence/destination                         |
| [CAEP et RISC](../../archive/apps/identity-service/src/identity.domains.auth.jwt.service.events.rs)                                              | Émission d’événements et dispatcher archivé                       | Absent                                                      | Différer interop externe ; besoin local de révocation d’abord, pas SSF complet prouvé            |
| [JWKS et signature locale/KMS](../../archive/apps/identity-service/src/identity.domains.auth.jwt.kms.rs)                                         | Adaptateur Scaleway et signature PS256                            | Clé locale RS256, JWKS interne                              | Conserver gestion active ; concevoir rotation, KMS seulement avec coût/indisponibilité maîtrisés |
| [Rate limits et anti credential-stuffing](../../archive/apps/identity-service/src/identity.domains.auth.login.throttle.rs)                       | Règles IP/compte/paire/tenant                                     | Pas parcours HTTP actif protégé                             | Prioritaire avant login public ; quotas bornés sans blocage abusif du compte                     |
| [Mot de passe exposé via signal edge](../../archive/apps/identity-service/src/identity.domains.auth.exposed_credentials.rs)                      | Header Cloudflare + marqueur proxy                                | Non attesté                                                 | Conditionnel ; preuve d’origine edge et budget, header utilisateur jamais fiable                 |
| [PoW progressif](../../archive/apps/identity-service/src/identity.domains.auth.pow.rs)                                                           | Challenge PostgreSQL et difficulté selon risque                   | Absent d’Identity actif                                     | Conditionnel après mesure ; plafonner CPU client, éviter Watch/TV par défaut                     |
| [Honeypot, temps de formulaire et signature JS](../../archive/apps/identity-service/src/identity.domains.auth.bot_guard.rs)                      | Code heuristique                                                  | Absent d’Identity actif                                     | Signal seulement ; accessibilité, autofill et provenance à revoir                                |
| [Scoring bot, tarpit et délais](../../archive/apps/identity-service/src/identity.domains.auth.bot_response.rs)                                   | Code de délai borné                                               | Non attesté                                                 | Réécrire dans politique Risk ; limiter concurrence et coût serveur                               |
| [Fingerprint / User-Agent / Client Hints / géographie](../../archive/apps/identity-service/src/identity.domains.auth.risk.signals.rs)            | Heuristiques et projections                                       | Pas intégration Identity→Risk attestée                      | Minimiser et déplacer corrélation vers Trust/Risk ; jamais facteur d’authentification            |
| [Audit, redaction et événements](../../archive/apps/identity-service/src/identity.domains.audit.redaction.rs)                                    | Code                                                              | Audit/outbox internes actifs                                | Conserver socle actuel ; reprendre règles de redaction utiles et tester fuite de secrets         |
| [Protection HTTP et sorties SSRF](../../archive/apps/identity-service/src/identity.http.outbound.rs)                                             | Validation réseau ; routeur borne taille/délai                    | Infra partagée, parcours futurs à vérifier                  | Adapter aux callbacks/JWKS distants ; DNS/redirections à tester                                  |
| [Idempotence HTTP](../../archive/apps/identity-service/src/identity.http.middleware.idempotency.rs)                                              | Middleware archivé                                                | Pas middleware Identity actif attesté                       | Adapter aux mutations concernées ; pas substitut à usage unique des codes                        |
| [Authz tenant / RBAC](../../archive/apps/identity-service/src/identity.domains.authz.service.tenant.rs)                                          | Code et modèle historique                                         | Account propriétaire actuel                                 | Ne pas réintroduire ownership Identity ; expliciter contrats Account                             |
| [SSO obligatoire par domaine](../../archive/apps/identity-service/src/identity.domains.auth.sso_policy.rs)                                       | Politique SQL présente                                            | Absent                                                      | Différer : Enterprise hors V1 ; ne prouve pas login fédéré                                       |
| [Fédération OIDC externe / SAML / SCIM](../../archive/apps/identity-service/src/identity.domains.federation.mod.rs)                              | 22 références locales absentes ; configuration Enterprise séparée | Absent                                                      | Non récupérable comme module complet ; Enterprise hors V1                                        |

## Écarts qui interdisent une reprise directe

1. **Fédération incomplète.** Les 22 chemins déclarés par
   `identity.domains.federation.mod.rs` sont absents de son dossier. Les
   [RPC Enterprise](../../archive/apps/enterprise-service/src/enterprise.grpc.federation.providers.rs)
   configurent des fournisseurs OIDC/SAML ; cela ne prouve pas la présence des
   traitements de connexion. Aucun moteur OpenID Federation avec Entity
   Statements et chaînes de confiance n'a été identifié dans les sources
   archivées recherchées. Ne pas confondre ces deux sens de « fédération ».
2. **Assurance WebAuthn surévaluée.** `webauthn.assurance.rs` affecte AAL3 à
   une clé non backup-eligible, selon notamment un kind déclaré. Cela ne
   démontre pas une clé non exportable ni les autres exigences AAL3. Revoir
   aussi les options JSON modifiées après génération de l'état de cérémonie.
   [Exigences NIST](https://pages.nist.gov/800-63-4/sp800-63b.html).
3. **Métadonnées plus larges que la preuve.** `metadata.rs` annonce pairwise,
   tandis que [l'émetteur ID token](../../archive/apps/identity-service/src/identity.domains.auth.jwt.service.issue.rs)
   utilise `sub = user_id`. La recherche pairwise/sector_identifier n'a trouvé
   que l'annonce dans ce service. Retirer cette annonce ou livrer sa sémantique.
4. **Token Exchange spécifique Cloud.** L'audience Cloud est imposée ; le
   refresh, l'assurance et Device Authorization consultent aussi les anciennes
   frontières Cloud. Réécrire pour Identity/Account/Billing sans importer Cloud.
5. **Schéma et dépendances incompatibles.** Les archives utilisent principals,
   tenants, oauth_clients, Redis et AppState ; le runtime utilise les tables
   `identity_*` et PgPool. Les crates Redis/DPoP partagées existent encore,
   mais leur présence n'implique ni intégration ni coût admissible. Le
   [Cargo archivé](../../archive/apps/identity-service/Cargo.toml) comporte aussi
   des chemins relatifs qui ne correspondent plus à son emplacement.
6. **Profil high_assurance non universel.** Il refuse les clients publics.
   L'activer globalement contredirait le client public Account Web proposé.
   Un nom de profil et des tests unitaires ne certifient pas FAPI ; comparer
   exigence par exigence puis exécuter une suite de conformité.
   [FAPI 2.0](https://openid.net/specs/fapi-security-profile-2_0.html).
7. **Résilience à reconstruire explicitement.** Refresh/challenges et sessions
   dépendent de Redis ; KMS/Twilio ajoutent des dépendances réseau. Définir les
   timeouts, limites de concurrence, reprise et comportement de refus avant
   activation. Aucun fallback ne doit contourner une validation de sécurité.

Les recherches n'ont pas identifié de parcours complet de connexion sociale
Google/Apple, magic-link, CIBA, JARM ou OpenID Federation dans les sources ciblées.
L'email de récupération n'est pas un mécanisme général de login magic-link.
Les écrans archivés et déclarations de modules ne changent pas ce niveau de preuve.

## Frontières et menaces à traiter lors du portage

| Frontière / menace                                            | Contrôle et preuve attendus                                                             |
| ------------------------------------------------------------- | --------------------------------------------------------------------------------------- |
| Navigateur → Identity : phishing, rejeu, énumération          | WebAuthn, PKCE, transaction unique, réponses non révélatrices, limites multi-dimensions |
| Identity → Account/Billing : confusion d'audience, privilèges | Audience exacte, scopes réduits, autorisation de ressource, délégation explicite        |
| Utilisateur → récupération : contournement MFA                | Preuve adaptée, codes consommés atomiquement, notification et révocation                |
| Edge → Identity : signaux forgés                              | Nettoyage des headers entrants et authentification de la frontière proxy                |
| Identity → Risk : indisponibilité et faux positifs            | Politique par action, données minimales, délais/quota ; signal ≠ facteur                |
| Service → Redis/DB/KMS : perte d'état ou compromission        | Rotation, restauration testée, anti-rejeu durable et absence de bypass                  |
| Identity → callbacks externes : SSRF, retry storm             | Destinations autorisées, validation réseau, retries bornés et idempotence               |

## Ordre recommandé

- **Lot A — socle avant tout login web :** protocole Code/PKCE, état hébergé,
  OIDC fidèle, scopes/audiences, cookies/CSRF/Origin, throttling, audit. Conserver
  les primitives actives plutôt que restaurer les anciennes tables.
- **Lot B — authentification forte :** passkeys, enrollment, récupération,
  fraîcheur du step-up et politique opérateur. TOTP en option de compatibilité.
- **Lot C — cycle de vie :** refresh si retenu, révocation, rotation des clés et
  logout intersites. Décider du stockage atomique sans Redis obligatoire.
- **Lot D — durcissement ciblé :** PAR et DPoP après cohérence serveur/client ;
  délégation machine si nécessaire. DPoP protège contre le rejeu d'un token
  volé seul, pas contre du code XSS capable d'utiliser la clé du client.
  [RFC 9449](https://www.rfc-editor.org/rfc/rfc9449.html).
- **Plus tard :** Device Authorization avec TV/Watch, JAR/mTLS/KMS selon menace
  et budget, événements externes CAEP/RISC et clients tiers.
- **Hors V1 actuelle :** SAML/SCIM/SSO Enterprise, fédération multilatérale,
  politiques et autorisations Cloud/Drive. SMS et scoring invasif ne sont pas
  ajoutés sans besoin mesuré et validation coût/accessibilité.

## Conditions communes de reprise

Pour chaque capacité retenue : écrire son contrat cible, mapper ses données,
extraire seulement la logique pure utile et réimplémenter l'orchestration avec
les dépendances précises. Reprendre les scénarios adversariaux archivés dans
les tests actifs : rejeu/concurrence, mauvaise origine, mauvaise audience,
révocation, faux signal proxy, expiration, compte tiers, panne du store.

Validation future : checks Nx ciblés, `cargo check --workspace` pour tout Rust,
tests avec vraies migrations puis parcours navigateur. Pas de service public
partiellement sécurisé ; pas de revendication AAL/FAPI sans preuves complètes.
Chaque nouvelle dépendance respecte le plafond global 30 EUR TTC/mois, en
incluant les appels variables, le stockage et l'exploitation.

Ce relevé ne modifie aucun runtime et n'exécute pas les tests archivés : les
références manquantes empêchent de traiter l'archive comme une base compilable
intacte. Il complète la [préparation des frontends](frontend-platform-preparation.md).
