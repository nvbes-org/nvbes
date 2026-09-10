# Preuve navigateur du protocole Identity

## Périmètre observé

Le 2026-09-10, Chromium 152.0.7977.83 a exécuté le SDK TypeScript réel contre
les binaires Identity, Account et Billing, avec PostgreSQL et migrations réelles.
Deux clients OAuth enregistrés utilisent deux origines HTTPS distinctes ;
Identity utilise `localhost`, les sites clients `127.0.0.1`, pour traverser
également une frontière de site lors de la navigation.

Le scénario vérifie :

- PAR avec une preuve DPoP et une transaction PKCE/nonce dans sessionStorage ;
- navigation vers Identity, connexion et consentement JSON avec cookies/CSRF ;
- retour sur chaque site, récupération de la même clé privée non exportable
  depuis IndexedDB et vérification signée de l'ID token par le SDK ;
- suppression de la transaction et de sa clé temporaire, puis rotation du
  refresh token avec conservation de la clé DPoP en mémoire ;
- clés distinctes entre les deux sites, accès Account/Billing avec DPoP,
  refus du même token en Bearer et refus de l'audience API sur UserInfo ;
- session Identity reconnue lors de la deuxième navigation, cookie
  `__Host-nvbes-session` Secure/HttpOnly/SameSite=Lax et absent des sites clients ;
- refus CORS effectivement appliqué par Chromium depuis une origine non inscrite ;
- logout Identity, refus des deux API et des refresh, effacement des sessions SDK.

## Rejouer

Prérequis : Docker actif, image locale `postgres:17-alpine`, OpenSSL, dépendances
du workspace et runtime Playwright fournissant un objet `Browser`.

```bash
pnpm nx run identity-service:test:https-browser-fixture
```

Cette cible prépare l'environnement et affiche l'URL HTTPS du premier client.
Elle ne constitue pas à elle seule un test réussi. Charger ensuite la fonction
`verifyBrowserJourney` de
`apps/identity-service/tests/runtime-browser-journey.mjs` dans le runtime
Playwright et lui passer l'objet Browser et cette URL. La fonction vérifie les
assertions, retourne un résumé sans jeton ni mot de passe et ferme son contexte.
La preuve enregistrée a été exécutée avec le connecteur Playwright local.

Le scénario complémentaire `verifyBrowserWebauthn` dans
`apps/identity-service/tests/runtime-browser-webauthn.mjs` utilise les mêmes
arguments et une clé CTAP2 virtuelle dans Chromium. Il passe enregistrement et
step-up via le SDK, logout, nouvelle autorisation, connexion découvrable sans
mot de passe et second step-up, puis échange OIDC et accès Account avec DPoP.
La session doit être absente avant chaque connexion ; le sujet vérifié reste
celui du compte synthétique. Le compteur de signatures confirme trois assertions.
Ce scénario a été exécuté avec succès le 2026-09-10, sans réponse serveur simulée,
sans remise à zéro des quotas ni changement d'état en base pendant le parcours.
Il vérifie également la liste des clés, le renommage persistant, le refus HTTP 409
de révoquer le dernier facteur, puis l'enregistrement d'une seconde clé virtuelle.
La révocation de la première clé invalide la session qui l'a utilisée et le jeton
Account déjà émis ; le SDK observe ces refus par les vraies routes.

Le serveur de test se ferme après dix minutes, à la perte de son lanceur ou sur
SIGINT/SIGTERM. Le PID affiché permet aussi un arrêt explicite avec `kill -TERM`.
Il supprime ses
certificats, son conteneur PostgreSQL et ses processus enfants. Aucun identifiant
de production n'est hérité, aucune télémétrie n'est configurée et Stripe pointe
vers un port loopback fermé. Les données de connexion synthétiques sont servies
uniquement par les pages du test sur loopback et ne sont jamais journalisées.

## Limites de la preuve

Le certificat de test est signé par une autorité éphémère. Les clients Rust
utilisent `SSL_CERT_FILE` dans leurs seuls processus pour vérifier le TLS local.
Chromium accepte ce certificat via `ignoreHTTPSErrors` dans un contexte neuf ;
CORS, cookies et WebCrypto restent actifs. Cette exécution ne valide donc pas
la chaîne de certificats publique ni la configuration TLS de production.

Les pages sont un banc de test protocolaire, sans interface produit : les
actions de login et consentement utilisent fetch dans l'origine Identity.
Les sites Identity/Account, accessibilité, passkeys matérielles, Safari/Firefox,
mobile, backchannel logout, charge, restauration et budget restent à livrer ou
vérifier. La révocation observée ne démontre pas une notification de logout aux
interfaces inactives. Aucune certification OIDC/FAPI/AAL n'est revendiquée.
La clé CTAP2 virtuelle prouve l'intégration cryptographique et navigateur, pas
la compatibilité avec des clés physiques ou des fournisseurs de passkeys synchronisées.
L'exécution navigateur n'est pas encore un gate CI automatique.
